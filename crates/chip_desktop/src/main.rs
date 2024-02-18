use minifb::{Key, Window, WindowOptions};

use chip_core::{
    Cpu,
    globals::{SCREEN_WIDTH, SCREEN_HEIGHT}
};

const SCALING: usize = 4;
const W: usize = SCALING * SCREEN_WIDTH;
const H: usize = SCALING * SCREEN_HEIGHT;

fn main() {
    let ibm = include_bytes!("../../../.local/ibm.ch8");
    println!("CHIP-8");

    let mut cpu = Cpu::new();
    cpu.load_rom(0x200, ibm);

    let mut window = Window::new(
            "CHIP-8",
            W,
            H,
            WindowOptions::default()
        )
        .unwrap();

    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    while window.is_open() {
        if let Err(e) = cpu.step() {
            println!("{:?}", e);
        }
        let _ = window.update_with_buffer(&get_buffer(&cpu), W, H);
    }
}

fn get_buffer(cpu: &Cpu) -> [u32; W * H] {
    let mut buffer = [0u32; W * H];
    let input = cpu.get_display_buffer();

    for y in 0..SCREEN_HEIGHT {
        for x in 0..SCREEN_WIDTH/8 {
            for i in 0..8 {
                let val = input[y*SCREEN_WIDTH/8 + x] >> (7-i) & 0x01;
                for sy in 0..SCALING {
                    for sx in 0..SCALING {
                        let dy = y * SCALING + sy;
                        let dx = (8 * x + i) * SCALING + sx;
                        buffer[dy * W + dx] = val as u32 * 255;
                    }
                }
            }
        }
    }
    buffer
}
