pub const RAM_SIZE: usize = 4096;
pub const STACK_SIZE: usize = 16;
pub const REG_COUNT: usize = 16;

pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;
pub const SCREEN_BUFFER_SIZE: usize = SCREEN_WIDTH * SCREEN_HEIGHT / 8;

pub const FONT_ADDR: u16 = 0x0050;

// QUIRKS
pub const SHIFT_OP_USE_VY: bool = false;