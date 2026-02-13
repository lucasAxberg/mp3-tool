fn header_exists(file: &[u8]) -> bool {
    // Data must be atleast 10 bytes
    if file.len() < 10 { return false; }

    // Check if header matches format given by: https://id3.org/id3v2.3.0#ID3v2_header 
    file[0..3] == "ID3".bytes().collect::<Vec<u8>>() &&    // ID3
    file[3] == 3 &&                                        // Major ver
    file[4] == 0 &&                                        // Minor ver
    (0..5).map(|x| (1 << x) & file[5]).all(|x| x == 0) &&  // Only 3 flag bits allowed
    file[6..].iter().all(|x| *x < 128)                     // Size in sync-safe int
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
} 
