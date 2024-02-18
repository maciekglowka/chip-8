#[derive(Debug, PartialEq)]
pub enum ChipError {
    IllegalInst(u16),
    IllegalAddr(u16),
    IllegalReg(u8),
    StackOverflow,
}
