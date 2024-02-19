use crate::{
    display::Display,
    errors::ChipError,
    font::FONT,
    globals::{RAM_SIZE, STACK_SIZE, REG_COUNT, FONT_ADDR},
    utils::{u8_from_two, u16_from_two, u16_from_three}
};

pub struct Cpu {
    memory: [u8; RAM_SIZE],
    display: Display,
    v: [u8; REG_COUNT],
    pc: u16,
    i: u16,
    sp: usize,
    stack: [u16; STACK_SIZE],
    delay_timer: u8,
    sound_timer: u8,
    keys: [bool; 0x10],
    random_seed: u32,
    redraw: bool
}
impl Cpu {
    pub fn new() -> Self {
        let mut cpu = Cpu {
            memory: [0; RAM_SIZE],
            display: Display::new(),
            v: [0; REG_COUNT],
            pc: 0,
            i: 0,
            sp: 0,
            stack: [0; STACK_SIZE],
            delay_timer: 0,
            sound_timer: u8::MAX,
            keys: [false; 0x10],
            random_seed: 0x5321a409,
            redraw: false,
        };
        cpu.load(FONT_ADDR, &FONT);
        cpu
    }
    pub fn load_rom(&mut self, addr: u16, data: &[u8]) {
        self.load(addr, data);
        self.pc = addr;
    }
    fn load(&mut self, addr: u16, data: &[u8]) {
        let end = addr as usize + data.len();
        self.memory[addr as usize..end].copy_from_slice(data);
    }
    pub fn get_display_buffer(&self) -> &[u8] {
        self.display.get_buffer()
    }
    /// Checks and clears the redraw flag
    pub fn take_redraw(&mut self) -> bool {
        if self.redraw {
            self.redraw = false;
            return true;
        }
        false
    }
    // set initial state for the XORshift
    pub fn set_random_seed(&mut self, val: u32) {
        self.random_seed = val;
    }
    pub fn step(&mut self) -> Result<(), ChipError> {
        let op = self.get_current_opcode()?;
        self.pc += 2;
        match op {
            (0, 0, 0xE, 0) => {
                self.display.clear();
                self.redraw = true;
            },
            (0, 0, 0xE, 0xE) => self.pc = self.pop_stack()?,
            // machine subroutine -> ignored
            (0, _, _, _) => (),
            (1, n0, n1, n2) => self.pc = u16_from_three(n0, n1, n2),
            (2, n0, n1, n2) => {
                self.push_stack(self.pc)?;
                self.pc = u16_from_three(n0, n1, n2);
            },
            (3, x, n0, n1) => if *(self.get_reg(x)?) == u8_from_two(n0, n1) {
                self.pc += 2;
            },
            (4, x, n0, n1) => if *(self.get_reg(x)?) != u8_from_two(n0, n1) {
                self.pc += 2;
            },
            (5, x, y, 0) => if self.get_reg(x)? == self.get_reg(y)? {
                self.pc += 2;
            },
            (6, x, n0, n1) => self.set_reg(x, u8_from_two(n0, n1))?,
            (7, x, n0, n1) => {
                let val = self.get_reg(x)?.wrapping_add(u8_from_two(n0, n1));
                self.set_reg(x, val)?;
            },
            (8, x, y, 0) => self.set_reg(x, *self.get_reg(y)?)?,
            (8, x, y, 1) => self.set_reg(x, self.get_reg(x)? | self.get_reg(y)?)?,
            (8, x, y, 2) => self.set_reg(x, self.get_reg(x)? & self.get_reg(y)?)?,
            (8, x, y, 3) => self.set_reg(x, self.get_reg(x)? ^ self.get_reg(y)?)?,
            (8, x, y, 4) => {
                let (val, overflow) = self.get_reg(x)?.overflowing_add(*self.get_reg(y)?);
                self.set_reg(x, val)?;
                self.set_flag(overflow);
            },
            (8, x, y, 5) => {
                let (val, overflow) = self.get_reg(x)?.overflowing_sub(*self.get_reg(y)?);
                self.set_reg(x, val)?;
                self.set_flag(overflow);
            },
            (8, x, y, 6) => {
                // TODO configure op ambuguity
                let val = *self.get_reg(y)?;
                self.set_flag(val & 1 == 1);
                self.set_reg(x, val >> 1)?;
            },
            (8, x, y, 7) => {
                let (val, overflow) = self.get_reg(y)?.overflowing_sub(*self.get_reg(x)?);
                self.set_reg(x, val)?;
                self.set_flag(overflow);
            },
            (8, x, y, 0xE) => {
                // TODO configure op ambuguity
                let val = *self.get_reg(y)?;
                self.set_flag(val >> 7 == 1);
                self.set_reg(x, val << 1)?;
            },
            (9, x, y, 0) => if self.get_reg(x)? != self.get_reg(y)? {
                self.pc += 2;
            },
            (0xA, n0, n1, n2) => {
                self.i = u16_from_three(n0, n1, n2);
            },
            (0xB, n0, n1, n2) => {
                self.pc = u16_from_three(n0, n1, n2) + self.v[0] as u16;
            },
            (0xC, x, n0, n1) => {
                let r = self.random();
                self.set_reg(x, r & u8_from_two(n0, n1))?;
            },
            (0xD, x, y, n) => {
                if self.i + n as u16 >= self.memory.len() as u16 {
                    return Err(ChipError::IllegalAddr(self.i + n as u16));
                }
                let data = &self.memory[self.i as usize..self.i as usize + n as usize];
                let flag = self.display.blit_sprite(
                    *self.get_reg(x)? as usize,
                    *self.get_reg(y)? as usize,
                    data,
                    n as usize
                );
                self.set_flag(flag != 0);
                self.redraw = true;
            },
            (0xE, x, 9, 0xE) => if *self.get_key(*self.get_reg(x)?)? { self.pc += 2 },
            (0xE, x, 0xA, 1) => if !*self.get_key(*self.get_reg(x)?)? { self.pc += 2 },
            (0xF, x, 0, 7) => self.set_reg(x, self.delay_timer)?,
            (0xF, x, 0, 0xA) => {
                if let Some(pressed) = self.keys.iter().enumerate().find(|(_, a)| **a) {
                    self.set_reg(x, pressed.0 as u8)?;
                } else {
                    self.pc -= 2;
                }
            },
            (0xF, x, 1, 5) => self.delay_timer = *self.get_reg(x)?,
            (0xF, x, 1, 8) => self.sound_timer = *self.get_reg(x)?,
            (0xF, x, 1, 0xE) => self.i = self.i.wrapping_add(*self.get_reg(x)? as u16),
            (0xF, x, 2, 9) => self.i = FONT_ADDR + *self.get_reg(x)? as u16,
            (0xF, x, 3, 3) => {
                let val = *self.get_reg(x)?;
                self.memory[self.i as usize] = val / 100;
                self.memory[self.i as usize + 1] = val % 100 / 10;
                self.memory[self.i as usize + 2] = val % 10;
            },
            (0xF, x, 5, 5) => {
                for t in 0..=x {
                    self.memory[self.i as usize + t as usize] = *self.get_reg(t)?;
                }
            },
            (0xF, x, 6, 5) => {
                for t in 0..=x {
                    self.set_reg(t, self.memory[self.i as usize + t as usize])?;
                }
            },
            _ => return Err(ChipError::IllegalInst(u16_from_two(
                self.memory[self.pc as usize - 2],
                self.memory[self.pc as usize - 1]
            ))),
        };
        Ok(())
    }
    fn get_current_opcode(&self) -> Result<(u8, u8, u8, u8), ChipError> {
        let addr = self.pc as usize;
        if addr > RAM_SIZE - 2 {
            return Err(ChipError::IllegalAddr(self.pc))
        }
        Ok((
            self.memory[addr] >> 4,
            self.memory[addr] & 0x0F ,
            self.memory[addr + 1] >> 4,
            self.memory[addr + 1] & 0x0F,
        ))
    }
    fn get_reg(&self, i: u8) -> Result<&u8, ChipError> {
        self.v.get(i as usize).ok_or(ChipError::IllegalReg(i))
    }
    fn set_reg(&mut self, i: u8, val: u8) -> Result<(), ChipError> {
        *(self.v.get_mut(i as usize).ok_or(ChipError::IllegalReg(i))?) = val; 
        Ok(())
    }
    fn get_key(&self, i: u8) -> Result<&bool, ChipError> {
        self.keys.get(i as usize).ok_or(ChipError::IllegalKey(i))
    }
    fn push_stack(&mut self, val: u16) -> Result<(), ChipError> {
        self.stack[self.sp] = val;
        self.sp += 1;
        if self.sp >= STACK_SIZE { return Err(ChipError::StackOverflow) };
        Ok(())
    }
    fn pop_stack(&mut self) -> Result<u16, ChipError> {
        if self.sp == 0 { return Err(ChipError::StackUnderflow) }
        self.sp -= 1;
        Ok(self.stack[self.sp])
    }
    fn set_flag(&mut self, val: bool) {
        self.v[0xF] = if val { 1 } else { 0 };
    }
    fn random(&mut self) -> u8 {
        let mut val = self.random_seed;
        val ^= val << 13;
        val ^= val >> 17;
        val ^= val << 5;
        self.random_seed = val;
        val as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn get_opcode() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0xA4;
        cpu.memory[0x201] = 0xC3;
        assert!(cpu.get_current_opcode() == Ok((0xA, 0x4, 0xC, 0x3)));
    }
    #[test]
    fn get_opcode_illegal_addr() {
        let mut cpu = Cpu::new();
        cpu.pc = RAM_SIZE as u16 - 1;
        assert!(cpu.get_current_opcode() == Err(ChipError::IllegalAddr(cpu.pc)));
        cpu.pc = RAM_SIZE as u16;
        assert!(cpu.get_current_opcode() == Err(ChipError::IllegalAddr(cpu.pc)));
    }

    // OPCODES

    #[test]
    fn op_00e0() {
        let mut cpu = Cpu::new();
        cpu.display.load(&[0xFF; crate::globals::SCREEN_BUFFER_SIZE]);
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0x00;
        cpu.memory[0x201] = 0xE0;
        let _ = cpu.step();
        assert!(cpu.display.get_buffer() == &[0u8; crate::globals::SCREEN_BUFFER_SIZE]);
        assert!(cpu.pc == 0x202);
    }
    #[test]
    fn op_00ee() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0x00;
        cpu.memory[0x201] = 0xEE;
        let _ = cpu.push_stack(0x0232);
        let _ = cpu.step();
        assert!(cpu.pc == 0x232);
        assert!(cpu.sp == 0);
    }
    #[test]
    fn op_1nnn() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0x1a;
        cpu.memory[0x201] = 0x5f;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0a5f);
    }
    #[test]
    fn op_2nnn() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0x2a;
        cpu.memory[0x201] = 0x5f;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0a5f);
        assert!(cpu.stack[0] == 0x0202);
        assert!(cpu.sp == 1);
    }
    #[test]
    fn op_3xnn_skip() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[5] = 0xc3;
        cpu.memory[0x200] = 0x35;
        cpu.memory[0x201] = 0xc3;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0204);
    }
    #[test]
    fn op_3xnn_dont_skip() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[5] = 0xc4;
        cpu.memory[0x200] = 0x35;
        cpu.memory[0x201] = 0xc3;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0202);
    }
    #[test]
    fn op_4xnn_skip() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[5] = 0xc3;
        cpu.memory[0x200] = 0x45;
        cpu.memory[0x201] = 0xc5;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0204);
    }
    #[test]
    fn op_4xnn_dont_skip() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[5] = 0xc3;
        cpu.memory[0x200] = 0x45;
        cpu.memory[0x201] = 0xc3;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0202);
    }
    #[test]
    fn op_5xy0_skip() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[7] = 0xc3;
        cpu.v[9] = 0xc3;
        cpu.memory[0x200] = 0x57;
        cpu.memory[0x201] = 0x90;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0204);
    }
    #[test]
    fn op_5xy0_dont_skip() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[7] = 0xc3;
        cpu.v[9] = 0xa3;
        cpu.memory[0x200] = 0x57;
        cpu.memory[0x201] = 0x90;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0202);
    }
    #[test]
    fn op_6xnn() {
        let mut cpu = Cpu::new();
        cpu.v[2] = 0x12;
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0x62;
        cpu.memory[0x201] = 0xC5;
        let _ = cpu.step();
        assert!(cpu.v[2] == 0xC5);
        assert!(cpu.pc == 0x202);
    }
    #[test]
    fn op_7xnn() {
        let mut cpu = Cpu::new();
        cpu.v[4] = 0x12;
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0x74;
        cpu.memory[0x201] = 0xC3;
        let _ = cpu.step();
        assert!(cpu.v[4] == 0xC3 + 0x12);
        assert!(cpu.pc == 0x202);
    }
    #[test]
    fn op_7xnn_overflow() {
        let mut cpu = Cpu::new();
        cpu.v[8] = 0xF0;
        // set VF to something random
        cpu.v[0xF] = 0xA;
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0x78;
        cpu.memory[0x201] = 0x11;
        let _ = cpu.step();
        assert!(cpu.v[8] == 0x01);
        assert!(cpu.pc == 0x202);
        // assert VF not affected
        assert!(cpu.v[0xF] == 0xA);
    }
    #[test]
    fn op_8xy0() {
        let mut cpu = Cpu::new();
        cpu.v[4] = 0x12;
        cpu.v[2] = 0x0F;
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0x84;
        cpu.memory[0x201] = 0x20;
        let _ = cpu.step();
        assert!(cpu.v[4] == 0x0F);
        assert!(cpu.pc == 0x202);
    }
    #[test]
    fn op_8xy1() {
        let mut cpu = Cpu::new();
        cpu.v[4] = 0b00010000;
        cpu.v[2] = 0b00001000;
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0x84;
        cpu.memory[0x201] = 0x21;
        let _ = cpu.step();
        assert!(cpu.v[4] == 0b00011000);
        assert!(cpu.pc == 0x202);
    }
    #[test]
    fn op_8xy2() {
        let mut cpu = Cpu::new();
        cpu.v[4] = 0b00010010;
        cpu.v[2] = 0b00011001;
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0x84;
        cpu.memory[0x201] = 0x22;
        let _ = cpu.step();
        assert!(cpu.v[4] == 0b00010000);
        assert!(cpu.pc == 0x202);
    }
    #[test]
    fn op_8xy3() {
        let mut cpu = Cpu::new();
        cpu.v[4] = 0b00010010;
        cpu.v[2] = 0b00011001;
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0x84;
        cpu.memory[0x201] = 0x23;
        let _ = cpu.step();
        assert!(cpu.v[4] == 0b00001011);
        assert!(cpu.pc == 0x202);
    }
    #[test]
    fn op_8xy4() {
        let mut cpu = Cpu::new();
        cpu.v[5] = 0x20;
        cpu.v[0xA] = 0x32;
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0x85;
        cpu.memory[0x201] = 0xA4;
        let _ = cpu.step();
        assert!(cpu.v[5] == 0x52);
        assert!(cpu.v[0xF] == 0x00);
        assert!(cpu.pc == 0x202);
    }
    #[test]
    fn op_8xy4_overflow() {
        let mut cpu = Cpu::new();
        cpu.v[5] = 0xF0;
        cpu.v[0xA] = 0x11;
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0x85;
        cpu.memory[0x201] = 0xA4;
        let _ = cpu.step();
        assert!(cpu.v[5] == 0x01);
        assert!(cpu.v[0xF] == 0x01);
        assert!(cpu.pc == 0x202);
    }
    #[test]
    fn op_8xy5() {
        let mut cpu = Cpu::new();
        cpu.v[5] = 0x32;
        cpu.v[0xA] = 0x20;
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0x85;
        cpu.memory[0x201] = 0xA5;
        let _ = cpu.step();
        assert!(cpu.v[5] == 0x12);
        assert!(cpu.v[0xF] == 0x00);
        assert!(cpu.pc == 0x202);
    }
    #[test]
    fn op_8xy5_overflow() {
        let mut cpu = Cpu::new();
        cpu.v[5] = 0x32;
        cpu.v[0xA] = 0x33;
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0x85;
        cpu.memory[0x201] = 0xA5;
        let _ = cpu.step();
        assert!(cpu.v[5] == 0xFF);
        assert!(cpu.v[0xF] == 0x01);
        assert!(cpu.pc == 0x202);
    }
    #[test]
    fn op_8xy6_set() {
        let mut cpu = Cpu::new();
        cpu.v[0xA] = 0b10011001;
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0x85;
        cpu.memory[0x201] = 0xA6;
        let _ = cpu.step();
        assert!(cpu.v[5] == 0b01001100);
        assert!(cpu.v[0xA] == 0b10011001);
        assert!(cpu.v[0xF] == 0x01);
        assert!(cpu.pc == 0x202);
    }
    #[test]
    fn op_8xy6_clear() {
        let mut cpu = Cpu::new();
        cpu.v[0xA] = 0b10011000;
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0x85;
        cpu.memory[0x201] = 0xA6;
        let _ = cpu.step();
        assert!(cpu.v[5] == 0b01001100);
        assert!(cpu.v[0xA] == 0b10011000);
        assert!(cpu.v[0xF] == 0x00);
        assert!(cpu.pc == 0x202);
    }
    #[test]
    fn op_8xy7() {
        let mut cpu = Cpu::new();
        cpu.v[5] = 0x20;
        cpu.v[0xA] = 0x32;
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0x85;
        cpu.memory[0x201] = 0xA7;
        let _ = cpu.step();
        assert!(cpu.v[5] == 0x12);
        assert!(cpu.v[0xF] == 0x00);
        assert!(cpu.pc == 0x202);
    }
    #[test]
    fn op_8xy7_overflow() {
        let mut cpu = Cpu::new();
        cpu.v[5] = 0x33;
        cpu.v[0xA] = 0x32;
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0x85;
        cpu.memory[0x201] = 0xA7;
        let _ = cpu.step();
        assert!(cpu.v[5] == 0xFF);
        assert!(cpu.v[0xF] == 0x01);
        assert!(cpu.pc == 0x202);
    }
    #[test]
    fn op_8xye_set() {
        let mut cpu = Cpu::new();
        cpu.v[0xA] = 0b10011001;
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0x85;
        cpu.memory[0x201] = 0xAE;
        let _ = cpu.step();
        assert!(cpu.v[5] == 0b00110010);
        assert!(cpu.v[0xA] == 0b10011001);
        assert!(cpu.v[0xF] == 0x01);
        assert!(cpu.pc == 0x202);
    }
    #[test]
    fn op_8xye_clear() {
        let mut cpu = Cpu::new();
        cpu.v[0xA] = 0b00011001;
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0x85;
        cpu.memory[0x201] = 0xAE;
        let _ = cpu.step();
        assert!(cpu.v[5] == 0b00110010);
        assert!(cpu.v[0xA] == 0b00011001);
        assert!(cpu.v[0xF] == 0x00);
        assert!(cpu.pc == 0x202);
    }
    #[test]
    fn op_9xy0_skip() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[7] = 0xc3;
        cpu.v[9] = 0xc4;
        cpu.memory[0x200] = 0x97;
        cpu.memory[0x201] = 0x90;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0204);
    }
    #[test]
    fn op_9xy0_dont_skip() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[7] = 0xc3;
        cpu.v[9] = 0xc3;
        cpu.memory[0x200] = 0x97;
        cpu.memory[0x201] = 0x90;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0202);
    }
    #[test]
    fn op_annn() {
        let mut cpu = Cpu::new();
        cpu.i = 0x12;
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0xa2;
        cpu.memory[0x201] = 0xC5;
        let _ = cpu.step();
        assert!(cpu.i == 0x02C5);
        assert!(cpu.pc == 0x202);
    }
    #[test]
    fn op_bnnn() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[0] = 0x04;
        cpu.memory[0x200] = 0xb2;
        cpu.memory[0x201] = 0x10;
        let _ = cpu.step();
        assert!(cpu.pc == 0x214);
    }
    #[test]
    fn op_cxnn() {
        // testing random result ;)
        let mut cpu = Cpu::new();
        cpu.random_seed = 0x12325a5d;
        cpu.pc = 0x200;
        cpu.v[2] = 0x00;
        cpu.memory[0x200] = 0xc2;
        cpu.memory[0x201] = 0x15;
        let _ = cpu.step();
        assert!(cpu.v[2] != 0);
        assert!(cpu.pc == 0x202);
    }
    #[test]
    fn op_dxyn() {
        // based on I drawing from the IBM logo rom
        let mut cpu = Cpu::new();
        let mut rom = [0; 0xFF];
        let ins = [
            0x00, 0xe0,
            0xa2, 0x2a,
            0x60, 0x0c,
            0x61, 0x08,
            0xd0, 0x1f
        ];
        let data = [
            0xff, 0x00, 0xff, 0x00, 0x3c, 0x00, 0x3c, 0x00,
            0x3c, 0x00, 0x3c, 0x00, 0xff, 0x00, 0xff
        ];
        rom[0x0..0xA].copy_from_slice(&ins);
        rom[0x2a..0x39].copy_from_slice(&data);
        cpu.load_rom(0x200, &rom);
        for _ in 0..5 {
            let _ = cpu.step();
        }
        let buffer = cpu.display.get_buffer();
        assert!(cpu.v[0] == 0x0c);
        assert!(cpu.v[1] == 0x08);
        assert!(cpu.v[0xF] == 0);
        assert!(cpu.i == 0x22a);
        let start = (0x0c + 0x08 * 64) / 8;
        let row = crate::globals::SCREEN_WIDTH / 8;
        assert!(buffer[start] == 0b00001111);
        assert!(buffer[start + 1] == 0b11110000);
        assert!(buffer[start + row] == 0b00000000);
        assert!(buffer[start + row + 1] == 0b00000000);
        assert!(buffer[start + 2 * row] == 0b00001111);
        assert!(buffer[start + 2 * row + 1] == 0b11110000);
        assert!(buffer[start + 4 * row] == 0b00000011);
        assert!(buffer[start + 4 * row + 1] == 0b11000000);
    }
    #[test]
    fn op_dxyn_collision() {
        let mut cpu = Cpu::new();
        cpu.display.load(&[0xFF; crate::globals::SCREEN_BUFFER_SIZE]);
        let mut rom = [0; 0xFF];
        let ins = [
            0xa2, 0x20,
            0x60, 0x08,
            0x61, 0x00,
            0xd0, 0x11
        ];
        rom[0x0..0x8].copy_from_slice(&ins);
        // sprite data
        rom[0x20] = 0b11011101;
        cpu.load_rom(0x200, &rom);
        for _ in 0..4 {
            let _ = cpu.step();
        }
        let buffer = cpu.display.get_buffer();
        assert!(cpu.v[0xF] == 1);
        assert!(buffer[0] == 0b11111111);
        assert!(buffer[1] == 0b00100010);
    }
    #[test]
    fn op_ex9e_skip() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.keys[7] = true;
        cpu.v[3] = 0x07;
        cpu.memory[0x200] = 0xe3;
        cpu.memory[0x201] = 0x9e;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0204);
    }
    #[test]
    fn op_ex9e_dont_skip() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.keys[7] = false;
        cpu.v[3] = 0x07;
        cpu.memory[0x200] = 0xe3;
        cpu.memory[0x201] = 0x9e;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0202);
    }
    #[test]
    fn op_exa1_skip() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.keys[7] = false;
        cpu.v[3] = 0x07;
        cpu.memory[0x200] = 0xe3;
        cpu.memory[0x201] = 0xa1;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0204);
    }
    #[test]
    fn op_exa1_dont_skip() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.keys[7] = true;
        cpu.v[3] = 0x07;
        cpu.memory[0x200] = 0xe3;
        cpu.memory[0x201] = 0xa1;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0202);
    }
    #[test]
    fn op_fx07() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.delay_timer = 0x53;
        cpu.memory[0x200] = 0xf3;
        cpu.memory[0x201] = 0x07;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0202);
        assert!(cpu.v[3] == 0x53);
    }
    #[test]
    fn op_fx0a_wait() {
        let mut cpu = Cpu::new();
        cpu.keys = [false; 0x10];
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0xf5;
        cpu.memory[0x201] = 0x0a;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0200);
        assert!(cpu.v[5] == 0x00);
    }
    #[test]
    fn op_fx0a_go() {
        let mut cpu = Cpu::new();
        cpu.keys = [false; 0x10];
        cpu.keys[7] = true;
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0xf5;
        cpu.memory[0x201] = 0x0a;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0202);
        assert!(cpu.v[5] == 0x07);
    }
    #[test]
    fn op_fx15() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.delay_timer = 0x53;
        cpu.v[3] = 0x17;
        cpu.memory[0x200] = 0xf3;
        cpu.memory[0x201] = 0x15;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0202);
        assert!(cpu.delay_timer == 0x17);
    }
    #[test]
    fn op_fx18() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.sound_timer = 0x53;
        cpu.v[3] = 0x17;
        cpu.memory[0x200] = 0xf3;
        cpu.memory[0x201] = 0x18;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0202);
        assert!(cpu.sound_timer == 0x17);
    }
    #[test]
    fn op_fx1e() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.i = 0x0150;
        cpu.v[3] = 0x17;
        cpu.memory[0x200] = 0xf3;
        cpu.memory[0x201] = 0x1e;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0202);
        assert!(cpu.i == 0x167);
    }
    #[test]
    fn op_fx29() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.i = 0x0150;
        cpu.v[3] = 0x09;
        cpu.memory[0x200] = 0xf3;
        cpu.memory[0x201] = 0x29;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0202);
        assert!(cpu.i == FONT_ADDR + 0x09);
    }
    #[test]
    fn op_fx33() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.i = 0x0150;
        cpu.v[5] = 0x9c;
        cpu.memory[0x200] = 0xf5;
        cpu.memory[0x201] = 0x33;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0202);
        assert!(cpu.memory[0x0150] == 1);
        assert!(cpu.memory[0x0151] == 5);
        assert!(cpu.memory[0x0152] == 6);
    }
    #[test]
    fn op_fx55() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.i = 0x0150;
        cpu.v[0] = 0xcc;
        cpu.v[1] = 0x07;
        cpu.v[2] = 0xee;
        cpu.v[3] = 0x9c;
        cpu.v[4] = 0xfe;
        cpu.memory[0x200] = 0xf3;
        cpu.memory[0x201] = 0x55;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0202);
        assert!(cpu.memory[0x0150] == 0xcc);
        assert!(cpu.memory[0x0151] == 0x07);
        assert!(cpu.memory[0x0152] == 0xee);
        assert!(cpu.memory[0x0153] == 0x9c);
        assert!(cpu.memory[0x0154] == 0x00);
    }
    #[test]
    fn op_fx55_0() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.i = 0x0150;
        cpu.v[0] = 0xcc;
        cpu.v[1] = 0x07;
        cpu.memory[0x200] = 0xf0;
        cpu.memory[0x201] = 0x55;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0202);
        assert!(cpu.memory[0x0150] == 0xcc);
        assert!(cpu.memory[0x0151] == 0x00);
    }
    #[test]
    fn op_fx65() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.i = 0x0150;
        cpu.memory[0x0150] = 0xcc;
        cpu.memory[0x0151] = 0x07;
        cpu.memory[0x0152] = 0xee;
        cpu.memory[0x0153] = 0x9c;
        
        cpu.memory[0x200] = 0xf2;
        cpu.memory[0x201] = 0x65;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0202);
        assert!(cpu.v[0] == 0xcc);
        assert!(cpu.v[1] == 0x07);
        assert!(cpu.v[2] == 0xee);
        assert!(cpu.v[3] == 0x00);
    }
    #[test]
    fn op_fx65_0() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.i = 0x0150;
        cpu.memory[0x0150] = 0xcc;
        cpu.memory[0x0151] = 0x07;
        
        cpu.memory[0x200] = 0xf0;
        cpu.memory[0x201] = 0x65;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0202);
        assert!(cpu.v[0] == 0xcc);
        assert!(cpu.v[1] == 0x00);
    }
}
