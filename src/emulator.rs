use crate::{
    audio::{CpalDriver, CpalDriverTimeSource},
    video::VideoFrameSink,
};
use cosmic::iced::keyboard::key::Code as KeyCode;
use rustednes_common::{audio::AudioDriver, time::TimeSource};
use rustednes_core::{
    apu::SAMPLE_RATE as APU_SAMPLE_RATE,
    cartridge::Cartridge,
    cpu::CPU_FREQUENCY,
    input::Button,
    nes::Nes,
    ppu::{SCREEN_HEIGHT, SCREEN_WIDTH},
};
use std::error::Error;
use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

pub const CPU_CYCLE_TIME_NS: u64 = (1e9_f64 / CPU_FREQUENCY as f64) as u64 + 1;

pub struct Emulator {
    nes: Nes,
    audio_driver: CpalDriver,
    time_source: CpalDriverTimeSource,
    start_time_ns: u64,
    paused_time_ns: Option<u64>,
    emulated_cycles: u64,
    emulated_instructions: u64,
    // TODO: Handle save states.
    // state_manager: StateManager,
    keymap: HashMap<KeyCode, Button>,
    pixels: Vec<u8>,
    rom_path: PathBuf,
}

impl Emulator {
    pub fn new(rom: Cartridge, rom_path: PathBuf, keymap: HashMap<KeyCode, Button>) -> Self {
        let audio_driver = CpalDriver::new(APU_SAMPLE_RATE).unwrap();
        let time_source = audio_driver.time_source();
        tracing::info!("Audio sample rate: {}", audio_driver.sample_rate());
        let start_time_ns = time_source.time_ns();

        Self {
            nes: Nes::new(rom),
            audio_driver,
            time_source,
            start_time_ns,
            paused_time_ns: None,
            emulated_cycles: 0,
            emulated_instructions: 0,
            // state_manager: StateManager::new(rom_path, 10),
            keymap,
            pixels: vec![0u8; SCREEN_WIDTH * SCREEN_HEIGHT * 4],
            rom_path,
        }
    }

    pub fn tick(&mut self) {
        if self.paused_time_ns.is_some() {
            return;
        }

        let mut video_sink = VideoFrameSink::new(self.pixels.as_mut_slice());

        let target_time_ns = self.time_source.time_ns() - self.start_time_ns;
        let target_cycles = target_time_ns / CPU_CYCLE_TIME_NS;

        let mut audio_sink = self.audio_driver.sink();

        while self.emulated_cycles < target_cycles {
            let (cycles, _) = self.nes.step(&mut video_sink, &mut audio_sink);

            self.emulated_cycles += cycles as u64;
            self.emulated_instructions += 1;
        }
    }

    pub fn pause_emulation(&mut self) {
        self.paused_time_ns = Some(self.time_source.time_ns());
    }

    pub fn resume_emulation(&mut self) {
        if let Some(paused_time_ns) = self.paused_time_ns {
            let paused_duration_ns = self.time_source.time_ns() - paused_time_ns;
            self.start_time_ns += paused_duration_ns;
            self.paused_time_ns = None;
        }
    }

    pub fn reset(&mut self) {
        self.nes.reset();
        self.start_time_ns = self.time_source.time_ns();
        self.emulated_cycles = 0;
        self.emulated_instructions = 0;
    }

    pub fn load_rom(&mut self, rom: Cartridge, rom_path: PathBuf) {
        self.reset();
        self.nes = Nes::new(rom);
        self.rom_path = rom_path;
        // self.state_manager: StateManager::new(rom_path, 10),
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
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

    pub fn rom_path(&self) -> &Path {
        &self.rom_path
    }
}

pub fn load_rom(filename: &Path) -> Result<Cartridge, Box<dyn Error>> {
    let file = File::open(filename)?;

    let cartridge = match filename.extension() {
        Some(ext) if ext == "zip" => {
            tracing::info!("Unzipping {}", filename.display());
            let mut zip = zip::ZipArchive::new(&file)?;
            let mut zip_file = zip.by_index(0)?;
            Cartridge::load(&mut zip_file)?
        }
        _ => {
            let mut file = file;
            Cartridge::load(&mut file)?
        }
    };

    Ok(cartridge)
}
