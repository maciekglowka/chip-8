use crate::{
    display::Display,
    errors::ChipError,
    globals::{RAM_SIZE, STACK_SIZE, REG_COUNT},
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
    redraw: bool
}
impl Cpu {
    pub fn new() -> Self {
        Cpu {
            memory: [0; RAM_SIZE],
            display: Display::new(),
            v: [0; REG_COUNT],
            pc: 0,
            i: 0,
            sp: 0,
            stack: [0; STACK_SIZE],
            delay_timer: 0,
            sound_timer: u8::MAX,
            redraw: false
        }
    }
    pub fn load_rom(&mut self, addr: u16, data: &[u8]) {
        self.load(addr, data);
        self.pc = addr;
    }
    // pub fn run(&mut self) -> Result<(), ChipError> {
    //     loop {
    //         self.step()?;
    //     }
    // }
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
            (6, x, n0, n1) => {
                self.set_reg(x, u8_from_two(n0, n1))?;
            },
            (7, x, n0, n1) => {
                let val = self.get_reg(x)?.wrapping_add(u8_from_two(n0, n1));
                self.set_reg(x, val)?;
            },
            (0xA, n0, n1, n2) => {
                self.i = u16_from_three(n0, n1, n2);
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
                self.v[0xF] = if flag == 0 { 1 } else { 0 };
                self.redraw = true;
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
    fn blit_test() {
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
}
