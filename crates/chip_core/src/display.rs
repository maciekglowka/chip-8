use crate::globals::{SCREEN_WIDTH, SCREEN_HEIGHT, SCREEN_BUFFER_SIZE};

pub struct Display {
    // TODO do not use u8s
    buffer: [u8; SCREEN_BUFFER_SIZE]
}
impl Display {
    pub fn new() -> Self {
        Display {
            buffer: [0; SCREEN_BUFFER_SIZE]
        }
    }
    pub fn clear(&mut self) {
        self.buffer = [0; SCREEN_BUFFER_SIZE];
    }
    pub fn load(&mut self, data: &[u8; SCREEN_BUFFER_SIZE]) {
        self.buffer.copy_from_slice(data);
    }
    pub fn get_buffer(&self) -> &[u8; SCREEN_BUFFER_SIZE] {
        &self.buffer
    }
    pub fn blit_sprite(&mut self, x: usize, y: usize, data: &[u8], lines: usize) {
        let mut cur = x % SCREEN_WIDTH + y * SCREEN_WIDTH;
        for i in 0..lines {

        }
    }
    /// returns a collision flag
    fn blit_byte(&mut self, x: usize, y: usize, mut data: u8) -> u8 {
        let px = x + y * SCREEN_WIDTH;
        let offset = px % 8;
        let i = px / 8;
        if i >= SCREEN_BUFFER_SIZE { return 0 }
        let flag;

        if (x + 8) / SCREEN_WIDTH != 0 {
            data &= 0xFFu8 << offset;
        }

        if offset == 0 {
            flag = self.buffer[i] & data;
            self.buffer[i] ^= data;
        } else {
            let mut b = self.buffer[i] >> offset;
            self.buffer[i] ^= data >> offset;
            if i + 1 < SCREEN_BUFFER_SIZE {
                b |= self.buffer[i+1] >> (8 - offset);
                self.buffer[i + 1] ^= data << (8 - offset);
            }
            flag = b & data;
        }
        flag
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blit_byte() {
        let mut display = Display::new();
        let flag = display.blit_byte(8, 0, 0b10101011);
        assert!(flag == 0x0);
        assert!(display.buffer[0] == 0x0);
        assert!(display.buffer[1] == 0b10101011);
        assert!(display.buffer[2] == 0x0);
    }
    #[test]
    fn blit_byte_with_y() {
        let mut display = Display::new();
        let flag = display.blit_byte(8, 2, 0b10101011);
        let target = (8 + 2 * 64) / 8;
        assert!(flag == 0x0);
        assert!(display.buffer[target-1] == 0x0);
        assert!(display.buffer[target] == 0b10101011);
        assert!(display.buffer[target+1] == 0x0);
    }
    #[test]
    fn blit_byte_non_empty() {
        let mut display = Display::new();
        display.buffer[1] = 0b11011111;
        let flag = display.blit_byte(8, 0, 0b10111111);
        assert!(flag != 0x0);
        assert!(display.buffer[0] == 0x0);
        assert!(display.buffer[1] == 0b01100000);
        assert!(display.buffer[2] == 0x0);
    }
    #[test]
    fn blit_byte_unaligned() {
        let mut display = Display::new();
        let flag = display.blit_byte(2, 0, 0b10101011);
        assert!(flag == 0x0);
        assert!(display.buffer[0] == 0b00101010);
        assert!(display.buffer[1] == 0b11000000);
        assert!(display.buffer[2] == 0x0);
    }
    #[test]
    fn blit_byte_unaligned_with_y() {
        let mut display = Display::new();
        let flag = display.blit_byte(2, 2, 0b10101011);
        let target = (2 + 2 * 64) / 8;
        assert!(flag == 0x0);
        assert!(display.buffer[target] == 0b00101010);
        assert!(display.buffer[target+1] == 0b11000000);
        assert!(display.buffer[target+2] == 0x0);
    }
    #[test]
    fn blit_byte_unaligned_non_empty() {
        let mut display = Display::new();
        display.buffer[1] = 0b10111111;
        let flag = display.blit_byte(2, 0, 0b10101011);
        assert!(flag != 0x0);
        assert!(display.buffer[0] == 0b00101010);
        assert!(display.buffer[1] == 0b01111111);
        assert!(display.buffer[2] == 0x0);
    }
    #[test]
    fn blit_byte_trim_x() {
        let mut display = Display::new();
        let flag = display.blit_byte(59, 0, 0b10101011);
        let target = 60 / 8;
        assert!(flag == 0x0);
        assert!(display.buffer[target] == 0b00010101);
        assert!(display.buffer[target+1] == 0x0);
    }
    #[test]
    fn blit_byte_exceed_buffer() {
        let mut display = Display::new();
        let flag = display.blit_byte(SCREEN_WIDTH - 1, SCREEN_HEIGHT - 1, 0b10101011);
        assert!(flag == 0x0);
        assert!(display.buffer[SCREEN_BUFFER_SIZE - 1] == 0b00000001);
    }
    #[test]
    fn blit_byte_trim_x_non_empty() {
        let mut display = Display::new();
        let target = 60 / 8;
        display.buffer[target+1] = 0b11101110;
        let flag = display.blit_byte(59, 0, 0b10101011);
        assert!(flag == 0x0);
        assert!(display.buffer[target] == 0b00010101);
        assert!(display.buffer[target+1] == 0b11101110);
    }
}
