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
    sound_timer: u8
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
            sound_timer: u8::MAX
        }
    }
    pub fn load_rom(&mut self, addr: u16, data: &[u8]) {
        self.load(addr, data);
        self.pc = addr;
    }
    pub fn run(&mut self) -> Result<(), ChipError> {
        loop {
            self.step()?;
        }
    }
    fn load(&mut self, addr: u16, data: &[u8]) {
        let end = addr as usize + data.len();
        self.memory[addr as usize..end].copy_from_slice(data);
    }
    fn step(&mut self) -> Result<(), ChipError> {
        let op = self.get_current_opcode()?;
        self.pc += 2;
        match op {
            (0, 0, 0xE, 0) => self.display.clear(),
            // machine subroutine -> ignored
            (0, _, _, _) => (),
            (1, n0, n1, n2) => self.pc = u16_from_three(n0, n1, n2),
            (6, x, n0, n1) => {
                self.v[x as usize] = u8_from_two(n0, n1);
            },
            (7, x, n0, n1) => {
                self.v[x as usize] = self.v[x as usize].wrapping_add(u8_from_two(n0, n1));
            },
            (0xA, n0, n1, n2) => {
                self.i = u16_from_three(n0, n1, n2);
            },
            _ => return Err(ChipError::IllegalInst(u16_from_two(
                self.memory[self.pc as usize],
                self.memory[self.pc as usize + 1]
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
    fn op_1nnn() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.memory[0x200] = 0x1a;
        cpu.memory[0x201] = 0x5f;
        let _ = cpu.step();
        assert!(cpu.pc == 0x0a5f);
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
}
