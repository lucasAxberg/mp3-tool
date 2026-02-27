use std::fmt;
use std::io::Read;

#[derive(Clone, Debug)]
/// Errors for the sync-safe integer data type
enum SyncSafeError {
    /// The array of bytes used in convertion is the wrong length
    IncorrectLength(usize)
}

impl fmt::Display for SyncSafeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::IncorrectLength(length) => write!(f, "expected '4' given: '{}'", length)
        }
    }
}

#[derive(Clone, Debug)]
/// Errors for the ID3 data structure creation functions
enum ID3Error {
    /// Did not find a valid header in the 10 bytes read
    HeaderNotFound,
    /// Was not able to read the amount of bytes needed
    NotEnoughBytes,
}

impl fmt::Display for ID3Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::HeaderNotFound => write!(f, "Header not found in the given bytes"),
            Self::NotEnoughBytes => write!(f, "Not enough bytes to parse in reader")
        }
    }
}

/// A representation of a sync-safe integer
struct SyncSafe(u32);

impl From<[u8; 4]> for SyncSafe {
    fn from(value: [u8; 4]) -> Self {
        // Bit mask ignores most significant bit
        let bit_mask: u8 = 0b_01111111;

        // Iterate over bytes, mask + shift, then set bits in val 
        let mut val: u32 = 0;
        for i in 0..4 {
            let shift_offset: usize = 7 * (3-i);
            val |= ((value[i] & bit_mask) as u32) << shift_offset; 
        }
        Self(val)
    }
}

impl TryFrom<Vec<u8>> for SyncSafe {
    type Error = SyncSafeError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        // Throw IncorrectLength error when length is not 4
        if value.len() != 4 {
            return Err(SyncSafeError::IncorrectLength(value.len()));
        };

        // Move 4 bytes into array and create sync-safe int
        Ok(SyncSafe::from([value[0], value[1], value[2], value[3]]))
    }
}

impl TryFrom<&[u8]> for SyncSafe {
    type Error = SyncSafeError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        // Throw IncorrectLength error when length is not 4
        if value.len() != 4 {
            return Err(SyncSafeError::IncorrectLength(value.len()));
        };

        // Move 4 bytes into array and create sync-safe int
        Ok(SyncSafe::from([value[0], value[1], value[2], value[3]]))
    }
}

impl From<SyncSafe> for Vec<u8> {
    fn from(value: SyncSafe) -> Self {
        // Bit mask ignores most significant bit
        let bit_mask: u8 = 0b_01111111;

        // Loop 4 times, shifting + masking sync-safe to extract bytes
        let mut bytes: Vec<u8> = Vec::with_capacity(4);
        for i in 0..4 {
            let shift_offset: usize = 7 * (3-i);
            bytes.push(((value.0 >> shift_offset) as u8) & bit_mask);
        };

        bytes
    }
}

struct Header {
    identifier: [u8; 3],
    version: [u8; 2],
    flags: u8,
    size: SyncSafe
}

impl Header {
    fn read_from(reader: &mut impl Read) -> Result<Self, ID3Error> {
        // Read 10 bytes from reader, or return error if not enough bytes
        let mut bytes: [u8; 10] = [0; 10];
        reader.read_exact(&mut bytes).map_err(|_| ID3Error::NotEnoughBytes)?;

        // Return error if header does not match pattern of valid header
        if Self::valid_bytes(bytes) == false {
            return Err(ID3Error::HeaderNotFound);
        };
        
        Ok(Self {
            identifier: [bytes[0], bytes[1], bytes[2]],
            version: [bytes[3], bytes[4]],
            flags: bytes[5],
            size: SyncSafe::try_from(&bytes[6..10]).unwrap()
        })
    }

    fn valid_bytes(bytes: [u8; 10]) -> bool {
        // Checks if 10 bytes matches specification given at:
        // https://id3.org/id3v2.3.0#ID3v2_header
        bytes[0..3] == [0x49, 0x44, 0x33] &&
        bytes[3..5].iter().all(|x| x < &0xFF) &&
        bytes[5] & 0b_00011111 == 0b_00000000 &&
        bytes[6..10].iter().all(|x| x < &0x80)
    }

    fn unsynchronisation(&self) -> bool {
        // Check if bit 'a' in flag is set (%abc00000)
        self.flags & 0b_1000_0000 == 0b_1000_0000 
    }

    fn extended_header(&self) -> bool {
        // Check if bit 'b' in flag is set (%abc00000)
        self.flags & 0b_0100_0000 == 0b_0100_0000 
    }

    fn experimental(&self) -> bool {
        // Check if bit 'c' in flag is set (%abc00000)
        self.flags & 0b_0010_0000 == 0b_0010_0000 
    }
}

struct ExtendedHeader {
    size: [u8; 4],
    flags: [u8; 2],
    padding_size: [u8; 4],
    crc: Option<[u8; 4]>
}

impl ExtendedHeader {
    fn read_from(reader: &mut impl Read) -> Result<Self, ID3Error> {
        // Read first 10 bytes of extended header, which are same for both types
        let mut header: [u8; 10] = [0; 10];
        reader.read_exact(&mut header).map_err(|_| ID3Error::NotEnoughBytes)?;

        // Check size bytes and read crc if size is 10
        let crc = if header[3] == 10 {
            let mut crc_bytes: [u8; 4] = [0; 4];
            reader.read_exact(&mut crc_bytes).map_err(|_| ID3Error::NotEnoughBytes)?;
            Some(crc_bytes)
        } else {
            None
        };

        Ok(Self{
            size: [header[0], header[1], header[2], header[3]],
            flags: [header[4], header[5]],
            padding_size: [header[6], header[7], header[8], header[9]],
            crc
        })
    }

    fn size(&self) -> u32 {
        u32::from_be_bytes(self.size)
    }

    fn crc(&self) -> Option<u32>{
        Some(u32::from_be_bytes(self.crc?))
    }

    fn padding_size(&self) -> u32 {
        u32::from_be_bytes(self.padding_size)
    }
}

struct FrameHeader {
    frame_id: [u8; 4],
    size: [u8; 4],
    flags: [u8; 2]
}

impl FrameHeader {
    fn read_from(reader: &mut impl Read) -> Result<Self, ID3Error> {
        let mut bytes: [u8; 10] = [0; 10];
        reader.read_exact(&mut bytes).map_err(|_| ID3Error::NotEnoughBytes)?;

        Ok(Self{
            frame_id: [bytes[0], bytes[1], bytes[2], bytes[3]],
            size: [bytes[4], bytes[5], bytes[6], bytes[7]],
            flags: [bytes[8], bytes[9]]
        })
    }

    fn size(&self) -> u32 {
        u32::from_be_bytes(self.size)
    }

    fn id(&self) -> String {
        String::from_utf8(self.frame_id.to_vec()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn parse_sync_safe_from_valid_bytes() {
        assert_eq!(SyncSafe::from([0b_01101110, 0b_01101110, 0b_01101110, 0b_01101110]).0, 0b_00001101_11011011_10110111_01101110);
    }

    #[test]
    fn parse_sync_safe_from_invalid_bytes() {
        assert_eq!(SyncSafe::from([0b_11101110, 0b_11101110, 0b_11101110, 0b_11101110]).0, 0b_00001101_11011011_10110111_01101110);
    }

    #[test]
    fn parse_sync_safe_from_valid_vec_of_valid_bytes() {
        assert_eq!(SyncSafe::try_from(vec![0b_01101110, 0b_01101110, 0b_01101110, 0b_01101110]).unwrap().0, 0b_00001101_11011011_10110111_01101110)
    }

    #[test]
    fn parse_sync_safe_from_valid_vec_of_invalid_bytes() {
        assert_eq!(SyncSafe::try_from(vec![0b_11101110, 0b_11101110, 0b_11101110, 0b_11101110]).unwrap().0, 0b_00001101_11011011_10110111_01101110)
    }

    #[test]
    #[should_panic]
    fn parse_sync_safe_panics_from_invalid_vec() {
        SyncSafe::try_from(vec![0b_11101110, 0b_11101110, 0b_11101110]).unwrap();
    }

    #[test]
    fn parse_sync_safe_from_valid_slice_of_valid_bytes() {
        assert_eq!(SyncSafe::try_from([0b_01101110, 0b_01101110, 0b_01101110, 0b_01101110].as_slice()).unwrap().0, 0b_00001101_11011011_10110111_01101110)
    }

    #[test]
    fn parse_sync_safe_from_valid_slice_of_invalid_bytes() {
        assert_eq!(SyncSafe::try_from([0b_11101110, 0b_11101110, 0b_11101110, 0b_11101110].as_slice()).unwrap().0, 0b_00001101_11011011_10110111_01101110)
    }

    #[test]
    #[should_panic]
    fn parse_sync_safe_panics_from_invalid_slice() {
        SyncSafe::try_from([0b_11101110, 0b_11101110, 0b_11101110].as_slice()).unwrap();
    }

    #[test]
    fn vec_of_bytes_from_valid_sync_safe() {
        assert_eq!(Vec::<u8>::from(SyncSafe(0b_00001101_11011011_10110111_01101110)), vec![0b_01101110, 0b_01101110, 0b_01101110, 0b_01101110])
    }
    #[test]
    fn vec_of_bytes_from_invalid_sync_safe() {
        assert_eq!(Vec::<u8>::from(SyncSafe(0b_11111101_11011011_10110111_01101110)), vec![0b_01101110, 0b_01101110, 0b_01101110, 0b_01101110])
    }

    #[test]
    fn parse_header_from_valid_bytes() {
        let bytes: [u8; 10] = [0x49, 0x44, 0x33, 0x03, 0x00, 0x00, 0x00, 0x0B, 0x36, 0x47];
        Header::read_from(&mut bytes.as_slice()).unwrap();
    }

    #[test]
    #[should_panic]
    fn parse_header_from_invalid_bytes() {
        let bytes: [u8; 10] = [0x49, 0x43, 0x32, 0x03, 0xFF, 0xFF, 0x00, 0x81, 0x36, 0x47];
        Header::read_from(&mut bytes.as_slice()).unwrap();
    }

    #[test]
    fn valid_header_false_on_invalid_identifier() {
        let bytes: [u8; 10] = [0x48, 0x43, 0x32, 0x03, 0x00, 0x00, 0x00, 0x0B, 0x36, 0x47];
        assert!(!Header::valid_bytes(bytes));
    }

    #[test]
    fn valid_header_false_on_invalid_version() {
        let bytes: [u8; 10] = [0x49, 0x44, 0x33, 0xFF, 0x00, 0x00, 0x00, 0x0B, 0x36, 0x47];
        assert!(!Header::valid_bytes(bytes));
    }

    #[test]
    fn valid_header_false_on_invalid_flags() {
        let bytes: [u8; 10] = [0x49, 0x44, 0x33, 0x03, 0x00, 0xFF, 0x00, 0x0B, 0x36, 0x47];
        assert!(!Header::valid_bytes(bytes));
    }

    #[test]
    fn valid_header_false_on_invalid_size() {
        let bytes: [u8; 10] = [0x49, 0x44, 0x33, 0x03, 0x00, 0x00, 0x00, 0x0B, 0x80, 0x47];
        assert!(!Header::valid_bytes(bytes));
    }

    #[test]
    #[should_panic]
    fn parse_header_panics_from_invalid_reader_length() {
        let bytes: [u8; 9] = [0x49, 0x44, 0x33, 0x03, 0x00, 0x00, 0x00, 0x0B, 0x36];
        Header::read_from(&mut bytes.as_slice()).unwrap();
    }

    #[test]
    fn parse_header_from_too_many_bytes() {
        let bytes: [u8; 11] = [0x49, 0x44, 0x33, 0x03, 0x00, 0x00, 0x00, 0x0B, 0x36, 0x47, 0xFF];
        Header::read_from(&mut bytes.as_slice()).unwrap();
    }

    #[test]
    fn header_unsynchronisation_flag_is_set() {
        let bytes: [u8; 10] = [0x49, 0x44, 0x33, 0x03, 0x00, 0b_10000000, 0x00, 0x0B, 0x36, 0x47];
        let header = Header::read_from(&mut bytes.as_slice()).unwrap();
        assert!(header.unsynchronisation());
    }

    #[test]
    fn header_extended_header_flag_is_set() {
        let bytes: [u8; 10] = [0x49, 0x44, 0x33, 0x03, 0x00, 0b_01000000, 0x00, 0x0B, 0x36, 0x47];
        let header = Header::read_from(&mut bytes.as_slice()).unwrap();
        assert!(header.extended_header());
    }

    #[test]
    fn header_experimental_flag_is_set() {
        let bytes: [u8; 10] = [0x49, 0x44, 0x33, 0x03, 0x00, 0b_00100000, 0x00, 0x0B, 0x36, 0x47];
        let header = Header::read_from(&mut bytes.as_slice()).unwrap();
        assert!(header.experimental());
    }

    #[test]
    fn parse_extended_header_from_valid_bytes_without_crc() {
        let bytes: [u8; 10] = [0, 0, 0, 6, 0, 0, 0, 0, 0, 0];
        let ext = ExtendedHeader::read_from(&mut bytes.as_slice()).unwrap();
        assert_eq!((ext.size.to_vec(), ext.flags.to_vec(), ext.padding_size.to_vec(), ext.crc), (bytes[0..4].to_vec(), bytes[4..6].to_vec(), bytes[6..10].to_vec(), None));
    }

    #[test]
    fn parse_extended_header_from_valid_bytes_with_crc() {
        let bytes: [u8; 14] = [0, 0, 0, 10, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let ext = ExtendedHeader::read_from(&mut bytes.as_slice()).unwrap();
        assert_eq!((ext.size.to_vec(), ext.flags.to_vec(), ext.padding_size.to_vec(), ext.crc), (bytes[0..4].to_vec(), bytes[4..6].to_vec(), bytes[6..10].to_vec(), Some([bytes[10], bytes[11], bytes[12], bytes[13]])));
    }

    #[test]
    #[should_panic]
    fn extended_header_from_valid_bytes_with_crc_not_enough_bytes() {
        let bytes: [u8; 13] = [0, 0, 0, 10, 0x80, 0, 0, 0, 0, 0, 0, 0, 0];
        ExtendedHeader::read_from(&mut bytes.as_slice()).unwrap();
    }

    #[test]
    #[should_panic]
    fn extended_header_from_valid_bytes_without_crc_not_enough_bytes() {
        let bytes: [u8; 9] = [0, 0, 0, 6, 0, 0, 0, 0, 0];
        ExtendedHeader::read_from(&mut bytes.as_slice()).unwrap();
    }

    #[test]
    fn extended_header_from_valid_bytes_with_crc_too_many_bytes() {
        let bytes: [u8; 15] = [0, 0, 0, 10, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let ext = ExtendedHeader::read_from(&mut bytes.as_slice()).unwrap();
        assert_eq!((ext.size.to_vec(), ext.flags.to_vec(), ext.padding_size.to_vec(), ext.crc), (bytes[0..4].to_vec(), bytes[4..6].to_vec(), bytes[6..10].to_vec(), Some([bytes[10], bytes[11], bytes[12], bytes[13]])));
    }

    #[test]
    fn extended_header_from_valid_bytes_without_crc_too_many_bytes() {
        let bytes: [u8; 11] = [0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0];
        let ext = ExtendedHeader::read_from(&mut bytes.as_slice()).unwrap();
        assert_eq!((ext.size.to_vec(), ext.flags.to_vec(), ext.padding_size.to_vec(), ext.crc), (bytes[0..4].to_vec(), bytes[4..6].to_vec(), bytes[6..10].to_vec(), None));
    }

    #[test]
    fn extended_header_getter_functions() {
        let bytes: [u8; 10] = [0, 0, 0, 6, 0, 0, 0x01, 0x02, 0x03, 0x04];
        let ext = ExtendedHeader::read_from(&mut bytes.as_slice()).unwrap();
        assert_eq!((ext.size(), ext.padding_size(), ext.crc()), (6, 16909060, None));
    }

    #[test]
    fn frame_header_from_valid_bytes() {
        let bytes: [u8; 10] = [0x54, 0x49, 0x54, 0x32, 0x00, 0x00, 0x00, 0x25, 0x00, 0x00];
        FrameHeader::read_from(&mut bytes.as_slice()).unwrap();
    }

    #[test]
    fn frame_header_from_too_many_bytes() {
        let bytes: [u8; 11] = [0x54, 0x49, 0x54, 0x32, 0x00, 0x00, 0x00, 0x25, 0x00, 0x00, 0x00];
        FrameHeader::read_from(&mut bytes.as_slice()).unwrap();
    }

    #[test]
    fn frame_header_error_from_not_enough_bytes() {
        let bytes: [u8; 9] = [0x54, 0x49, 0x54, 0x32, 0x00, 0x00, 0x00, 0x25, 0x00];
        let frame_header = FrameHeader::read_from(&mut bytes.as_slice());
        match frame_header {
            Err(ID3Error::NotEnoughBytes) => assert!(true),
            _ => assert!(false)
        }
    }

    #[test]
    fn frame_header_size() {
        let bytes: [u8; 10] = [0x54, 0x49, 0x54, 0x32, 0x00, 0x00, 0x00, 0x25, 0x00, 0x00];
        let head = FrameHeader::read_from(&mut bytes.as_slice()).unwrap();
        assert_eq!(head.size(), 37)
    }

    #[test]
    fn frame_header_id() {
        let bytes: [u8; 10] = [0x54, 0x49, 0x54, 0x32, 0x00, 0x00, 0x00, 0x25, 0x00, 0x00];
        let head = FrameHeader::read_from(&mut bytes.as_slice()).unwrap();
        assert_eq!(head.id(), "TIT2".to_string())
    }
}
