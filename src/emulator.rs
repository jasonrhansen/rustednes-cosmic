use cosmic::iced::keyboard::key::Code as KeyCode;
use rustednes_common::{
    state::StateManager,
    time::{SystemTimeSource, TimeSource},
};
use rustednes_core::{
    cartridge::Cartridge,
    cpu::CPU_FREQUENCY,
    input::Button,
    nes::Nes,
    ppu::{SCREEN_HEIGHT, SCREEN_WIDTH},
    sink::{AudioSink, VideoSink, XRGB8888_PALETTE},
};
use std::{collections::HashMap, mem, path::PathBuf};

pub const CPU_CYCLE_TIME_NS: u64 = (1e9_f64 / CPU_FREQUENCY as f64) as u64 + 1;

pub struct Emulator {
    nes: Nes,
    time_source: SystemTimeSource,
    start_time_ns: u64,
    emulated_cycles: u64,
    emulated_instructions: u64,
    state_manager: StateManager,
    keymap: HashMap<KeyCode, Button>,
    pixels: &'static mut [u8],
}

impl Emulator {
    pub fn new(rom: Cartridge, rom_path: PathBuf, keymap: HashMap<KeyCode, Button>) -> Self {
        let time_source = SystemTimeSource {};
        let start_time_ns = time_source.time_ns();

        Self {
            nes: Nes::new(rom),
            time_source,
            start_time_ns,
            emulated_cycles: 0,
            emulated_instructions: 0,
            state_manager: StateManager::new(rom_path, 10),
            keymap,
            pixels: vec![0u8; SCREEN_WIDTH * SCREEN_HEIGHT * 4].leak(),
        }
    }

    pub fn tick(&mut self) {
        let mut video_sink = VideoFrameSink::new(self.pixels);

        let target_time_ns = self.time_source.time_ns() - self.start_time_ns;
        let target_cycles = target_time_ns / CPU_CYCLE_TIME_NS;

        while self.emulated_cycles < target_cycles {
            let (cycles, _) = self.nes.step(&mut video_sink, &mut NullAudioSink {});

            self.emulated_cycles += cycles as u64;
            self.emulated_instructions += 1;
        }
    }

    pub fn pixels(&self) -> &[u8] {
        self.pixels
    }

    pub fn key_down(&mut self, key_code: KeyCode) {
        self.set_button_pressed(key_code, true);
    }

    pub fn key_up(&mut self, key_code: KeyCode) {
        self.set_button_pressed(key_code, false);
    }

    fn set_button_pressed(&mut self, key_code: KeyCode, pressed: bool) {
        if let Some(button) = self.keymap.get(&key_code) {
            self.nes
                .interconnect
                .input
                .game_pad_1
                .set_button_pressed(*button, pressed)
        }
    }
}

pub struct NullAudioSink;

impl AudioSink for NullAudioSink {
    fn write_sample(&mut self, _frame: f32) {
        // Do nothing
    }

    fn samples_written(&self) -> usize {
        0
    }
}

pub type PixelBuffer = [u8];

pub struct VideoFrameSink<'a> {
    pixels: &'a mut PixelBuffer,
    frame_written: bool,
}

impl<'a> VideoFrameSink<'a> {
    pub fn new(pixels: &'a mut PixelBuffer) -> Self {
        VideoFrameSink {
            pixels,
            frame_written: false,
        }
    }
}

impl<'a> VideoSink for VideoFrameSink<'a> {
    fn write_frame(&mut self, frame_buffer: &[u8]) {
        for (i, palette_index) in frame_buffer.iter().enumerate() {
            let pixel = XRGB8888_PALETTE[*palette_index as usize];
            let offset = i * 4;

            self.pixels[offset] = (pixel >> 16) as u8;
            self.pixels[offset + 1] = (pixel >> 8) as u8;
            self.pixels[offset + 2] = pixel as u8;
            self.pixels[offset + 3] = 0x77;
        }
        self.frame_written = true;
    }

    fn frame_written(&self) -> bool {
        self.frame_written
    }

    fn pixel_size(&self) -> usize {
        mem::size_of::<u32>()
    }
}
