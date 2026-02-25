struct SyncSafe(u32);

impl From<[u8; 4]> for SyncSafe {
    fn from(value: [u8; 4]) -> Self {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::SyncSafe;


    #[test]
    fn parse_sync_safe_from_valid_bytes() {
        assert_eq!(SyncSafe::from([0b_01101110, 0b_01101110, 0b_01101110, 0b_01101110]).0, 0b_00001101_11011011_10110111_01101110);
    }

    #[test]
    fn parse_sync_safe_from_invalid_bytes() {
        assert_eq!(SyncSafe::from([0b_11101110, 0b_11101110, 0b_11101110, 0b_11101110]).0, 0b_00001101_11011011_10110111_01101110);
    }
}
