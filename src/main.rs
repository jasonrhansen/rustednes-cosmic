// SPDX-License-Identifier: MPL-2.0

mod app;
mod audio;
mod config;
mod emulator;
mod i18n;
mod video;

use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use cosmic::iced::Size;
use emulator::load_rom;
use rustednes_common::logger;
use rustednes_core::ppu::{SCREEN_HEIGHT, SCREEN_WIDTH};
use std::{error::Error, path::PathBuf};
use tracing::info;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Opt {
    /// The name of the ROM to load
    #[arg(name = "ROM")]
    rom_path: Option<PathBuf>,

    #[clap(flatten)]
    verbose: Verbosity<InfoLevel>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt: Opt = clap::Parser::parse();

    logger::initialize(&opt.verbose);

    // Get the system's preferred languages.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    // Enable localizations to be applied.
    i18n::init(&requested_languages);

    let titlebar_height = 49.0;

    // Settings for configuring the application window and iced runtime.
    let settings = cosmic::app::Settings::default()
        .size_limits(
            cosmic::iced::Limits::NONE
                .min_width(SCREEN_WIDTH as f32)
                .min_height(SCREEN_HEIGHT as f32 + titlebar_height),
        )
        .size(Size::new(
            SCREEN_WIDTH as f32 * 3.0,
            SCREEN_HEIGHT as f32 * 3.0 + titlebar_height,
        ));

    let rom = if let Some(rom_path) = &opt.rom_path {
        let rom = load_rom(&rom_path)?;
        info!("{:?}", rom);
        let rom_path = rom_path.to_path_buf();
        Some((rom, rom_path))
    } else {
        None
    };

    cosmic::app::run::<app::AppModel>(settings, app::Flags { rom })?;

    Ok(())
}
