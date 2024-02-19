use minifb::{Key, Window, WindowOptions};

use chip_core::{
    Cpu,
    globals::{SCREEN_WIDTH, SCREEN_HEIGHT}
};

const SCALING: usize = 8;
const W: usize = SCALING * SCREEN_WIDTH;
const H: usize = SCALING * SCREEN_HEIGHT;

fn main() {
    let ibm = include_bytes!("../../../.local/chip8-logo.ch8");
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

    // window.limit_update_rate(None);
    let mut buffer = [0u32; W * H];

    while window.is_open() {
        let start = std::time::Instant::now();
        if let Err(e) = cpu.step() {
            println!("{:?}", e);
        }
        if cpu.take_redraw() {
            read_buffer(&mut buffer, &cpu);
            let _ = window.update_with_buffer(&buffer, W, H);
        }
        std::thread::sleep(std::time::Duration::from_micros(1440).saturating_sub(start.elapsed()));
        println!("{:?}", start.elapsed());
    }
}

fn read_buffer(buffer: &mut [u32; W * H], cpu: &Cpu) {
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
}
