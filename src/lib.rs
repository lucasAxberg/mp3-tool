use std::fmt;

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
}
