#[derive(PartialEq)]
pub enum ChipError {
    IllegalInst(u16),
    IllegalAddr(u16),
    StackOverflow,
}
