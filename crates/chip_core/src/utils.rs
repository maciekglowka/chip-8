#[inline(always)]
pub fn u8_from_two(a: u8, b: u8) -> u8 {
    // assumes u4 inputs, but does not verify
    a << 4 | b
}

#[inline(always)]
pub fn u16_from_three(a: u8, b: u8, c: u8) -> u16 {
    // assumes u4 inputs, but does not verify
    (a as u16) << 8 | (b as u16) << 4 | (c as u16)
}

#[inline(always)]
pub fn u16_from_two(a: u8, b: u8) -> u16 {
    (a as u16) << 8 | b as u16
}
