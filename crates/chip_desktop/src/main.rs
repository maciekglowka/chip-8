use std::{
    num::NonZeroU32,
    rc::Rc
};
use winit::{
    event::{Event, WindowEvent, KeyEvent},
    dpi::PhysicalSize,
    event_loop::{EventLoop, ControlFlow},
    keyboard::KeyCode,
    window::WindowBuilder
};

use chip_core::{
    Cpu,
    globals::{SCREEN_WIDTH, SCREEN_HEIGHT}
};

mod audio;

const SCALING: usize = 8;
const W: usize = SCALING * SCREEN_WIDTH;
const H: usize = SCALING * SCREEN_HEIGHT;
const GAP_V: usize = 2;
const GAP_H: usize = 2;

const STEP_DELAY_SECONDS: f32 = 1. / 480.;
const TIMER_FACTOR: usize = 8;

fn main() {
    println!("CHIP-8");
    let mut audio_device = audio::get_device();
    if let Some(_) = &mut audio_device {
        println!("Got Audio Device");
    }

    let rom = include_bytes!("../../../.local/Brix.ch8");

    let mut cpu = Cpu::new();
    cpu.load_rom(0x200, rom);

    
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
    
    event_loop.set_control_flow(ControlFlow::Poll);
    
    let mut keys = [false; 0x10];
    let mut start = std::time::Instant::now();
    let mut timer = 0;

    event_loop.run(move |event, elwt| {
            match event {
                Event::WindowEvent { window_id, event: WindowEvent::Resized(size) } => {
                    let _ = surface.resize(
                        NonZeroU32::new(size.width).unwrap(),
                        NonZeroU32::new(size.height).unwrap(),
                    );
                },
                Event::WindowEvent { window_id, event: WindowEvent::RedrawRequested } => {
                    if start.elapsed().as_secs_f32() >= STEP_DELAY_SECONDS {
                        cpu.set_keys(keys);
                        
                        if let Err(e) = cpu.step() {
                            println!("{:?}", e);
                        }
                        let mut buffer = surface.buffer_mut().unwrap();
                        if cpu.take_redraw() {
                            // println!("{:?}", cpu.v[0xf]);
                            // let start = std::time::Instant::now();
                            read_buffer(&mut buffer, &cpu);
                            // println!("Redraw {}", start.elapsed().as_secs_f32());
                        }

                        timer += 1;
                        if timer > TIMER_FACTOR {
                            // update timers and buffer at 60Hz
                            cpu.decrease_timers();
                            timer = 0;
                            buffer.present().unwrap();

                            if let Some(device) = &mut audio_device {
                                if cpu.beeps() { device.beep() } else { device.stop() }
                            }
                        }
                        // println!("{} {}", 1. / start.elapsed().as_secs_f32(), start.elapsed().as_secs_f32());
                        start = std::time::Instant::now();
                    }
                },
                Event::WindowEvent { window_id, event: WindowEvent::KeyboardInput { event, .. } } => {
                    let KeyEvent { physical_key, state, .. } = event;
                    if let winit::keyboard::PhysicalKey::Code(code) = physical_key {
                        match code {
                            KeyCode::Digit1 => keys[1] = state.is_pressed(),
                            KeyCode::Digit2 => keys[2] = state.is_pressed(),
                            KeyCode::Digit3 => keys[3] = state.is_pressed(),
                            KeyCode::Digit4 => keys[0xC] = state.is_pressed(),
                            KeyCode::KeyQ => keys[4] = state.is_pressed(),
                            KeyCode::KeyW => keys[5] = state.is_pressed(),
                            KeyCode::KeyE => keys[6] = state.is_pressed(),
                            KeyCode::KeyR => keys[0xD] = state.is_pressed(),
                            KeyCode::KeyA => keys[7] = state.is_pressed(),
                            KeyCode::KeyS => keys[8] = state.is_pressed(),
                            KeyCode::KeyD => keys[9] = state.is_pressed(),
                            KeyCode::KeyF => keys[0xE] = state.is_pressed(),
                            KeyCode::KeyZ => keys[0xA] = state.is_pressed(),
                            KeyCode::KeyX => keys[0] = state.is_pressed(),
                            KeyCode::KeyC => keys[0xB] = state.is_pressed(),
                            KeyCode::KeyV => keys[0xF] = state.is_pressed(),
                            _ => ()
                        }
                    }
                },
                Event::WindowEvent { window_id, event: WindowEvent::CloseRequested } => {
                    elwt.exit();
                },
                Event::AboutToWait => {
                    window.request_redraw();
                },
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
                let slice = [val; SCALING - GAP_V];
                let dx = (8 * x + i) * SCALING;
                for sy in GAP_H..SCALING {
                    let dy = y * SCALING + sy;
                    let start = dy * W + dx;
                    buffer[start + GAP_V..start + SCALING].copy_from_slice(&slice);
                }
            }
        }
    }
}
