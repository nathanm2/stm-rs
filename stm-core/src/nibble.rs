pub fn swap_nibbles(value: u64, nibble_sz: usize) -> u64 {
    let mut v = value.swap_bytes();
    v = ((v & 0xF0F0F0F0F0F0F0F0) >> 4) | ((v & 0x0F0F0F0F0F0F0F0F) << 4);
    v >> (64 - (4 * nibble_sz))
}

#[test]
fn test_swap_nibbles() {
    assert_eq!(0x1, swap_nibbles(0x1, 1));
    assert_eq!(0x21, swap_nibbles(0x12, 2));
    assert_eq!(0x210, swap_nibbles(0x012, 3));
    assert_eq!(0x21F, swap_nibbles(0xFFFF12, 3));
    assert_eq!(0xfedcba98765432, swap_nibbles(0x0123456789abcdef, 14));
    assert_eq!(0xfedcba987654321, swap_nibbles(0x0123456789abcdef, 15));
    assert_eq!(0xfedcba9876543210, swap_nibbles(0x0123456789abcdef, 16));
}
