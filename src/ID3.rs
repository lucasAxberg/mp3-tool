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
    bytes: Vec<u8>,
    index: usize
}

impl Default for Reader {
    fn default() -> Self {
        Self {bytes: Vec::new(), index: 0}   
    }
}

impl Reader {
    fn load(mut self, bytes: &[u8]) -> Self {
        self.bytes.append(&mut bytes.to_vec());
        self
    }

    fn skip_n_bytes(&mut self, n: usize) -> (){
        self.index = (self.index + n).min(self.bytes.len())
    }

    fn read_n_bytes(&mut self, n: usize) -> Vec<u8> {
        let mut read_bytes = Vec::new();
        for i in 0..n {
            let Some(byte) = self.bytes.get(self.index) else {
                break;
            };
            read_bytes.push(*byte);
            self.index += 1;
        }
        read_bytes
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
        let length: u64 = (0..4).map(|x| {(bytes[x] as u64) << 3-x}).sum();
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
        let mut reader = Reader::default().load(&[1, 2, 3, 4, 5]);
        assert_eq!(reader.read_n_bytes(3), vec![1, 2, 3]);
    }

    #[test]
    fn read_bytes_out_of_bounds() {
        let mut reader = Reader::default().load(&[1, 2, 3, 4, 5]);
        assert_eq!(reader.read_n_bytes(6), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn skip_bytes_in_bounds() {
        let mut reader = Reader::default().load(&[1, 2, 3, 4, 5]);
        reader.skip_n_bytes(3);
        assert_eq!(reader.index, 3);
    }

    #[test]
    fn skip_bytes_out_of_bounds() {
        let mut reader = Reader::default().load(&[1, 2, 3, 4, 5]);
        reader.skip_n_bytes(7);
        assert_eq!(reader.index, 5);
    }

    #[test]
    fn read_bytes_out_of_bounds_index() {
        let mut reader = Reader::default().load(&[1, 2, 3, 4, 5]);
        let _ = reader.read_n_bytes(7);
        assert_eq!(reader.index, 5);
    }

    #[test]
    fn construct_header() {
        let header = Header::from_bytes(&[0x49, 0x44, 0x33, 0x03, 0x00, 0xE0, 0x00, 0x08, 0x2e, 0x37]).unwrap();
        assert_eq!((header.major_ver, header.minor_ver, header.flags, header.size), (3, 0, 0b_11100000, [0, 8, 46, 55]));
    }

    #[test]
    fn header_sync_safe_size() {
        let header = Header::from_bytes(&[0x49, 0x44, 0x33, 0x03, 0x00, 0xE0, 0x00, 0x08, 0x2e, 0x37]).unwrap();
        assert_eq!(header.size(), 137015);
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
} 
