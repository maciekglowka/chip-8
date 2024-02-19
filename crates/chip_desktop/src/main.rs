use std::{
    num::NonZeroU32,
    rc::Rc
};
use winit::{
    event::{Event, WindowEvent},
    dpi::PhysicalSize,
    event_loop::{EventLoop, ControlFlow},
    window::WindowBuilder
};

use chip_core::{
    Cpu,
    globals::{SCREEN_WIDTH, SCREEN_HEIGHT}
};

const SCALING: usize = 8;
const W: usize = SCALING * SCREEN_WIDTH;
const H: usize = SCALING * SCREEN_HEIGHT;
const STEP_DELAY_MICROS: u128 = 1440;

fn main() {
    let ibm = include_bytes!("../../../.local/ibm.ch8");
    println!("CHIP-8");

    let mut cpu = Cpu::new();
    cpu.load_rom(0x200, ibm);

    
    // let mut buffer = [0u32; W * H];

    let event_loop = EventLoop::new().unwrap();
    let window = Rc::new(
        WindowBuilder::new().with_inner_size(
            PhysicalSize::new(W as u32, H as u32)
        )
        .with_resizable(false)
        .build(&event_loop).unwrap()
    );
    let context = softbuffer::Context::new(window.clone()).unwrap();
    let mut surface = softbuffer::Surface::new(&context, window.clone()).unwrap();
    let mut start = std::time::Instant::now();

    event_loop.set_control_flow(ControlFlow::Poll);

    event_loop.run(move |event, elwt| {
            match event {
                Event::WindowEvent { window_id, event: WindowEvent::Resized(size) } => {
                    let _ = surface.resize(
                        NonZeroU32::new(size.width).unwrap(),
                        NonZeroU32::new(size.height).unwrap(),
                    );
                },
                Event::WindowEvent { window_id, event: WindowEvent::RedrawRequested } => {
                    if let Err(e) = cpu.step() {
                        println!("{:?}", e);
                    }
                    if cpu.take_redraw() {
                        let mut buffer = surface.buffer_mut().unwrap();
                        let start = std::time::Instant::now();
                        read_buffer(&mut buffer, &cpu);
                        println!("Redraw {}", start.elapsed().as_secs_f32());
                        buffer.present().unwrap();
                    }
                    // println!("{} {}", 1. / start.elapsed().as_secs_f32(), start.elapsed().as_secs_f32());
                    start = std::time::Instant::now();
                },
                Event::WindowEvent { window_id, event: WindowEvent::CloseRequested } => {
                    elwt.exit();
                },
                Event::AboutToWait => {
                    if start.elapsed().as_micros() >= STEP_DELAY_MICROS {
                        window.request_redraw();
                    }
                }
                _ => ()
            }
        }).unwrap();

}

fn read_buffer<'a, D, W>(buffer: &mut softbuffer::Buffer<'a, D, W>, cpu: &Cpu)
where D: winit::raw_window_handle::HasDisplayHandle, W: winit::raw_window_handle::HasWindowHandle {
    let input = cpu.get_display_buffer();

    for y in 0..SCREEN_HEIGHT {
        for x in 0..SCREEN_WIDTH/8 {
            for i in 0..8 {
                let val = (input[y*SCREEN_WIDTH/8 + x] >> (7-i) & 0x01) as u32 * 255;
                let slice = [val; SCALING];
                let dx = (8 * x + i) * SCALING;
                for sy in 0..SCALING {
                    let dy = y * SCALING + sy;
                    let start = dy * W + dx;
                    buffer[start..start + SCALING].copy_from_slice(&slice);
                }
            }
        }
    }
}
