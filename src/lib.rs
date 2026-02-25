use std::fmt;

#[derive(Clone, Debug)]
enum SyncSafeError {
    IncorrectLength(usize)
}

impl fmt::Display for SyncSafeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::IncorrectLength(length) => write!(f, "expected '4' given: '{}'", length)
        }
    }
}

struct SyncSafe(u32);

impl From<[u8; 4]> for SyncSafe {
    fn from(value: [u8; 4]) -> Self {
        let mut val: u32 = 0;
        for i in 0..4 {
            let bit_mask: u8 = 0b_01111111;
            let shift_offset: usize = 7 * (3-i);
            val |= ((value[i] & bit_mask) as u32) << shift_offset; 
        }
        Self(val)
    }
}

impl TryFrom<Vec<u8>> for SyncSafe {
    type Error = SyncSafeError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() != 4 {
            return Err(SyncSafeError::IncorrectLength(value.len()));
        };

        Ok(SyncSafe::from([value[0], value[1], value[2], value[3]]))
    }
}

impl TryFrom<&[u8]> for SyncSafe {
    type Error = SyncSafeError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        todo!()
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
}
