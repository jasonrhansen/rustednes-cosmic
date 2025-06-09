// SPDX-License-Identifier: MPL-2.0

mod app;
mod config;
mod i18n;

use rustednes_common::logger;
use rustednes_core::cartridge::*;

use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use tracing::info;

use std::error::Error;
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Opt {
    /// The name of the ROM to load
    #[arg(name = "ROM")]
    rom_path: PathBuf,

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

    // Settings for configuring the application window and iced runtime.
    let settings = cosmic::app::Settings::default().size_limits(
        cosmic::iced::Limits::NONE
            .min_width(360.0)
            .min_height(180.0),
    );

    let rom = load_rom(&opt.rom_path)?;
    info!("{:?}", rom);
    let rom_path = opt.rom_path.to_path_buf();

    cosmic::app::run::<app::AppModel>(
        settings,
        app::Flags {
            rom: Some(rom),
            rom_path,
        },
    )?;

    Ok(())
}

fn load_rom(filename: &Path) -> Result<Cartridge, Box<dyn Error>> {
    let file = File::open(filename)?;

    let cartridge = match filename.extension() {
        Some(ext) if ext == "zip" => {
            info!("Unzipping {}", filename.display());
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
