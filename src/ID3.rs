use std::fs::File;
use std::io;
use std::io::BufReader;
use std::io::prelude::*;
use std::io::{Error, ErrorKind};

fn utf16_from_bytes(bytes: &[u8]) -> String {
    let bom = ((bytes[0] as u16) << 8) + bytes[1] as u16;
    let normal_order = if bom == 65534 {
        true
    } else if bom == 65279 {
        false
    } else {
        return String::new();
    };

    let mut string = String::new();
    for i in (2..bytes.len()).step_by(2) {
        if bytes[i] + bytes[i+1] == 0 {
            break;
        }

        let (first, second): (u16, u16) = if !normal_order {
            (bytes[i] as u16, bytes[i+1] as u16)
        } else {
            (bytes[i+1] as u16, bytes[i] as u16)
        };

        let utf_val = (first << 8) + second;
        string.push_str(&String::from_utf16_lossy(&[utf_val]));
    };

    string
}

fn ascii_from_bytes(bytes: &[u8]) -> String {
    let mut string = String::new();
    for byte in bytes {
        if *byte == 0 {
            break;
        }
        string.push(*byte as char);
    }
    string
}

fn string_from_bytes(bytes: &[u8]) -> Option<String>{
    let mut string = String::new();
    for byte in bytes {
        if !byte.is_ascii() {
            return None;
        }
        string.push(*byte as char);
    }
    Some(string)
}

fn header_exists(file: &[u8]) -> bool {
    // Data must be atleast 10 bytes
    if file.len() < 10 { return false; }

    // Check if header matches format given by: https://id3.org/id3v2.3.0#ID3v2_header 
    file[0..3] == "ID3".bytes().collect::<Vec<u8>>() &&    // ID3
    file[3] == 3 &&                                        // Major ver
    file[4] == 0 &&                                        // Minor ver
    (0..5).map(|x| (1 << x) & file[5]).all(|x| x == 0) &&  // Only 3 flag bits allowed
    file[6..10].iter().all(|x| *x < 128)                   // Size in sync-safe int
}

struct Reader {
    reader: BufReader<File>,
}

impl Reader {
    fn from_file(filename: &str) -> io::Result<Self>{
        let file = File::open(filename)?;
        let reader = BufReader::new(file);
        Ok(Self{
            reader 
        })
    }

    fn skip_n_bytes(&mut self, n: usize) -> io::Result<()>{
        self.reader.seek_relative(n as i64)
    }

    fn read_n_bytes(&mut self, n: usize) -> io::Result<Vec<u8>> {
        let mut buf: Vec<u8> = vec![0; n];
        self.reader.read_exact(&mut buf)?;
        Ok(buf)
    }

}

struct Header {
    major_ver: u8,
    minor_ver: u8,
    flags: u8,
    size: [u8; 4]
}

impl Header {
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        // Return none if no valid header
        if header_exists(bytes) == false {
            return None;
        }

        Some(Self{
            major_ver: bytes[3],
            minor_ver: bytes[4],
            flags: bytes[5],
            size: [bytes[6], bytes[7], bytes[8], bytes[9]],
        })
    }

    fn from_reader(reader: &mut Reader) -> io::Result<Self> {
        let bytes = reader.read_n_bytes(10)?;

        if header_exists(&bytes) == false {
            return Err(Error::new(ErrorKind::InvalidData, "File contains no ID3 header"));
        }
        
        Ok(Self {
            major_ver: bytes[3],
            minor_ver: bytes[4],
            flags: bytes[5],
            size: [bytes[6], bytes[7], bytes[8], bytes[9]],
        })
    }

    fn size(&self) -> u64 {
        (0..4).map(|x| { (self.size[x] as u64) << 7*(3-x) }).sum()
    }

    fn unsynchronisation(&self) -> bool {
        // Check if first flag bit is set
        (self.flags & 0b_10000000) >> 7 == 1
    }

    fn extended_header(&self) -> bool {
        // Check if second flag bit is set
        (self.flags & 0b_01000000) >> 6 == 1
    }

    fn experimental(&self) -> bool {
        // Check if third flag bit is set
        (self.flags & 0b_00100000) >> 5 == 1
    }
}

struct ExtendedHeader {
    size: [u8; 4],
    flags: [u8; 2],
    padding_size: [u8; 4],
    crc: Option<[u8; 4]>
}

impl ExtendedHeader {
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        // Skip if note enough bytes to get len
        if bytes.len() < 4{
            return None;
        }

        // Skip if not enough bytes for entire extended header
        let length: u64 = (0..4).map(|x| {(bytes[x] as u64) << 8*(3-x)}).sum();
        println!("{length}");
        if (bytes.len() as u64) < length + 4 {
            return None;
        }

        // Get CRC if it exists
        let crc = if length == 10 {
            Some([bytes[10], bytes[11], bytes[12], bytes[13]])
        } else {
            None
        };
        
        // Create and return extended header
        Some(Self{
            size: [bytes[0], bytes[1], bytes[2], bytes[3]],
            flags: [bytes[4], bytes[5]],
            padding_size: [bytes[6], bytes[7], bytes[8], bytes[9]],
            crc
        })
    }

    fn from_reader(reader: &mut Reader) -> io::Result<Self> {
        let size = reader.read_n_bytes(4)?;
        let more: u64 = (0..4).map(|x| {(size[x] as u64) << 8*(3-x)}).sum();
        let remaining = reader.read_n_bytes(more as usize)?;

        // Get CRC if header is big enough
        let crc = if more == 10 {
            Some([remaining[6], remaining[7], remaining[8], remaining[9]])
        } else {
            None
        };

        Ok(Self{
            size: [size[0], size[1], size[2], size[3]],
            flags: [remaining[0], remaining[1]],
            padding_size: [remaining[2], remaining[3], remaining[4], remaining[5]],
            crc
        })
    }

    fn padding_size(&self) -> u64 {
        (0..4).map(|i| {(self.padding_size[i] as u64) << 8*(3-i)}).sum()
    }

    fn size(&self) -> u64 {
        (0..4).map(|i| {(self.size[i] as u64) << 8*(3-i)}).sum()
    }

    fn has_padding(&self) -> bool {
        (self.flags[0] & 0b_10000000) >> 7 == 1
    }
}

struct Frame {
    id: [u8; 4],
    size: [u8; 4],
    flags: [u8; 2],
    data: Vec<u8>,
}

impl Frame {
    fn from_reader(reader: &mut Reader) -> io::Result<Self> {
        let header = reader.read_n_bytes(10)?;
        let size: u64 = (0..4).map(|x| {(header[4+x] as u64) << 8*(3-x)}).sum();
        let data = reader.read_n_bytes(size as usize)?;

        Ok(Self{
            id: [header[0], header[1], header[2], header[3]],
            size: [header[4], header[5], header[6], header[7]],
            flags: [header[8], header[9]],
            data
        })
    }

    fn id(&self) -> String {
        string_from_bytes(&self.id).unwrap()
    }

    fn size(&self) -> u64 {
        (0..4).map(|x| {(self.size[x] as u64) << 8*(3-x)}).sum()
    }

    // TODO: Flag helper functions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_header() {
        assert!(header_exists(&[0x49, 0x44, 0x33, 0x03, 0x00, 0xE0, 0x00, 0x08, 0x2e, 0x37]))
    }
    #[test]
    fn invalid_header() {
        assert!(!header_exists(&[0x49, 0x44, 0x33, 0x03, 0x00, 0x01, 0x00, 0x08, 0x2e, 0x37]))
    }

    #[test]
    fn read_bytes_in_bounds() {
        let mut reader = Reader::from_file("test/Polygondwanaland.mp3").unwrap();
        let bytes = reader.read_n_bytes(3).unwrap();
        assert_eq!(bytes, vec![0x49, 0x44, 0x33]);
    }

    #[test]
    fn skip_bytes_in_bounds() {
        let mut reader = Reader::from_file("test/Polygondwanaland.mp3").unwrap();
        reader.skip_n_bytes(3).unwrap();
        let bytes = reader.read_n_bytes(3).unwrap();
        assert_eq!(bytes, vec![0x03, 0x00, 0x00]);
    }

    #[test]
    fn construct_header() {
        let mut reader = Reader::from_file("test/Polygondwanaland.mp3").unwrap();
        let header = Header::from_reader(&mut reader).unwrap();
        assert_eq!((header.major_ver, header.minor_ver, header.flags, header.size), (3, 0, 0b_00000000, [0x00, 0x0b, 0x36, 0x47]));
    }

    #[test]
    fn header_sync_safe_size() {
        let mut reader = Reader::from_file("test/Polygondwanaland.mp3").unwrap();
        let header = Header::from_reader(&mut reader).unwrap();
        assert_eq!(header.size(), 187207);
    }

    #[test]
    fn header_flag_parsing() {
        let header = Header::from_bytes(&[0x49, 0x44, 0x33, 0x03, 0x00, 0xE0, 0x00, 0x08, 0x2e, 0x37]).unwrap();
        assert_eq!((header.unsynchronisation(), header.extended_header(), header.experimental()), (true, true, true));
    }

    #[test]
    fn construct_extended_header() {
        let header = ExtendedHeader::from_bytes(&[0x00, 0x00, 0x00, 0x0A, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0xDE, 0xAD, 0xBE, 0xEF ]).unwrap();
    }

    #[test]
    fn extended_header_size() {
        let header = ExtendedHeader::from_bytes(&[0x00, 0x00, 0x00, 0x0A, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0xDE, 0xAD, 0xBE, 0xEF ]).unwrap();
        assert_eq!(header.size(), 10);
    }

    #[test]
    fn padding_size() {
        let header = ExtendedHeader::from_bytes(&[0x00, 0x00, 0x00, 0x0A, 0x80, 0x00, 0x00, 0x00, 0x00, 0x80, 0xDE, 0xAD, 0xBE, 0xEF ]).unwrap();
        assert_eq!(header.padding_size(), 128);
    }

    #[test]
    fn padding_exists() {
        let header = ExtendedHeader::from_bytes(&[0x00, 0x00, 0x00, 0x0A, 0x80, 0x00, 0x00, 0x00, 0x00, 0x80, 0xDE, 0xAD, 0xBE, 0xEF ]).unwrap();
        assert_eq!(header.has_padding(), true);
    }

    #[test]
    fn bytes_to_string() {
        let bytes = [0x54, 0x49, 0x54, 0x32];
        assert_eq!(string_from_bytes(&bytes), Some("TIT2".to_string()));
    }

    #[test]
    fn bytes_to_utf16() {
        let bytes = [0xFF, 0xFE, 0x4C, 0x00, 0x69, 0x00, 0x62, 0x00, 0x62, 0x00, 0x79, 0x00, 0x20, 0x00, 0x44, 0x00, 0x65, 0x00, 0x43, 0x00, 0x61, 0x00, 0x6D, 0x00, 0x70, 0x00, 0x00, 0x00];
        assert_eq!(utf16_from_bytes(&bytes), "Libby DeCamp".to_string());
    }

    #[test]
    fn bytes_to_ascii() {
        let bytes = [0x43, 0x61, 0x73, 0x74, 0x6C, 0x65, 0x20, 0x52, 0x61, 0x74, 0x00];
        assert_eq!(ascii_from_bytes(&bytes), "Castle Rat".to_string());
    }
} 
