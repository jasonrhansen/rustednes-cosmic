// SPDX-License-Identifier: MPL-2.0

use crate::config::Config;
use crate::emulator::{load_rom, Emulator};
use crate::fl;
use cosmic::app::context_drawer;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::keyboard::key::{Code as KeyCode, Physical};
use cosmic::iced::keyboard::{Event as KeyEvent, Modifiers};
use cosmic::iced::{event, window, Alignment, Event, Length, Subscription};
use cosmic::iced_core::image;
use cosmic::prelude::*;
use cosmic::widget::{self, menu};
use cosmic::{cosmic_theme, theme};
use rfd::AsyncFileDialog;
use rustednes_core::cartridge::Cartridge;
use rustednes_core::input::Button;
use rustednes_core::ppu::{SCREEN_HEIGHT, SCREEN_WIDTH};
use std::collections::HashMap;
use std::path::PathBuf;
const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    core: cosmic::Core,
    context_page: ContextPage,
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    config: Config,
    emulator: Option<Emulator>,
    opening_file: bool,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    OpenRepositoryUrl,
    ToggleContextPage(ContextPage),
    UpdateConfig(Config),
    LaunchUrl(String),
    OpenFileDialog,
    OpenFileResult(Option<PathBuf>),
    KeyDown(Modifiers, KeyCode),
    KeyUp(Modifiers, KeyCode),
    Tick,
    ToggleEmulation,
    ResetEmulation,
}

#[derive(Default)]
pub struct Flags {
    pub rom: Option<(Cartridge, PathBuf)>,
}

/// Create a COSMIC application from the app model
impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = Flags;

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "com.github.jasonrhansen.rustednes-cosmic";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(core: cosmic::Core, flags: Self::Flags) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            key_binds: HashMap::new(),
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|context| match Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((errors, config)) => {
                        for why in errors {
                            tracing::error!(%why, "error loading app config");
                        }

                        config
                    }
                })
                .unwrap_or_default(),
            emulator: flags
                .rom
                .map(|(rom, rom_path)| Emulator::new(rom, rom_path, AppModel::keymap())),
            opening_file: false,
        };

        let command = app.update_title();

        (app, command)
    }

    fn header_start(&self) -> Vec<Element<Self::Message>> {
        let mut menu_trees = vec![
            menu::Tree::with_children(
                menu::root(fl!("file")),
                menu::items(
                    &self.key_binds,
                    vec![menu::Item::Button(
                        fl!("open-rom"),
                        None,
                        MenuAction::OpenFile,
                    )],
                ),
            ),
            menu::Tree::with_children(
                menu::root(fl!("view")),
                menu::items(
                    &self.key_binds,
                    vec![menu::Item::Button(fl!("about"), None, MenuAction::About)],
                ),
            ),
        ];

        if let Some(emulator) = &self.emulator {
            menu_trees.push(menu::Tree::with_children(
                menu::root(fl!("emulation")),
                menu::items(
                    &self.key_binds,
                    vec![
                        menu::Item::Button(
                            if emulator.is_paused() {
                                fl!("unpause")
                            } else {
                                fl!("pause")
                            },
                            None,
                            MenuAction::ToggleEmulation,
                        ),
                        menu::Item::Button(fl!("reset"), None, MenuAction::ResetEmulation),
                    ],
                ),
            ));
        }

        let menu_bar = menu::bar(menu_trees);

        vec![menu_bar.into()]
    }

    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => context_drawer::context_drawer(
                self.about(),
                Message::ToggleContextPage(ContextPage::About),
            )
            .title(fl!("about")),
        })
    }

    fn view(&self) -> Element<Self::Message> {
        widget::responsive(|size| {
            let main_element: Element<Self::Message> = if let Some(emulator) = &self.emulator {
                let image_handle = image::Handle::from_rgba(
                    SCREEN_WIDTH as u32,
                    SCREEN_HEIGHT as u32,
                    emulator.pixels().to_vec(),
                );

                let screen_ratio = SCREEN_WIDTH as f32 / SCREEN_HEIGHT as f32;
                let widget_ratio = size.width / size.height;

                let (width, height) = if screen_ratio > widget_ratio {
                    (
                        size.width,
                        size.width * (SCREEN_HEIGHT as f32 / SCREEN_WIDTH as f32),
                    )
                } else {
                    (screen_ratio * size.height, size.height)
                };
                widget::image(image_handle)
                    .width(width)
                    .height(height)
                    .into()
            } else {
                // widget::button(fl!("open-rom"), Message::OpenFileDialog)
                widget::list_column().into()
            };

            widget::column()
                .push(
                    widget::row()
                        .push(main_element)
                        .height(Length::Fill)
                        .align_y(Vertical::Center),
                )
                .width(Length::Fill)
                .align_x(Horizontal::Center)
                .into()
        })
        .into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            // Watch for application configuration changes.
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| {
                    for why in update.errors {
                        tracing::error!(?why, "app config error");
                    }

                    Message::UpdateConfig(update.config)
                }),
            event::listen_with(|event, status, _window_id| match event {
                Event::Keyboard(KeyEvent::KeyPressed {
                    physical_key: Physical::Code(code),
                    modifiers,
                    ..
                }) => match status {
                    event::Status::Ignored => Some(Message::KeyDown(modifiers, code)),
                    event::Status::Captured => None,
                },
                Event::Keyboard(KeyEvent::KeyReleased {
                    physical_key: Physical::Code(code),
                    modifiers,
                    ..
                }) => match status {
                    event::Status::Ignored => Some(Message::KeyUp(modifiers, code)),
                    event::Status::Captured => None,
                },
                _ => None,
            }),
            window::frames().map(|_| Message::Tick),
        ])
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::OpenRepositoryUrl => {
                _ = open::that_detached(REPOSITORY);
            }
            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    // Close the context drawer if the toggled context page is the same.
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    // Open the context drawer to display the requested context page.
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }
            }
            Message::UpdateConfig(config) => {
                self.config = config;
            }
            Message::LaunchUrl(url) => match open::that_detached(&url) {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("failed to open {url:?}: {err}");
                }
            },
            Message::OpenFileDialog => {
                if !self.opening_file {
                    if let Some(emulator) = &mut self.emulator {
                        emulator.pause_emulation();
                    }
                    self.opening_file = true;
                    return Task::future(async {
                        let file = AsyncFileDialog::new()
                            .add_filter("NES ROM file", &["nes", "rom"])
                            .pick_file()
                            .await;

                        cosmic::Action::App(Message::OpenFileResult(
                            file.map(|f| f.path().to_path_buf()),
                        ))
                    });
                }
            }
            Message::OpenFileResult(path_buf) => {
                self.opening_file = false;
                if let Some(emulator) = &mut self.emulator {
                    emulator.resume_emulation();
                }

                if let Some(rom_path) = path_buf {
                    if let Ok(rom) = load_rom(&rom_path) {
                        if let Some(emulator) = &mut self.emulator {
                            emulator.load_rom(rom, rom_path);
                        } else {
                            self.emulator = Some(Emulator::new(rom, rom_path, AppModel::keymap()))
                        }
                    } else {
                        tracing::error!("error loading rom");
                        // TODO: Show error message to user.
                    }
                }

                return self.update_title();
            }
            Message::KeyDown(_modifiers, key_code) => {
                if let Some(emulator) = &mut self.emulator {
                    emulator.key_down(key_code);
                }
            }
            Message::KeyUp(_modifiers, key_code) => {
                if let Some(emulator) = &mut self.emulator {
                    emulator.key_up(key_code);
                }
            }
            Message::Tick => {
                if let Some(emulator) = &mut self.emulator {
                    emulator.tick();
                }
            }
            Message::ToggleEmulation => {
                if let Some(emulator) = &mut self.emulator {
                    emulator.toggle_paused();
                }
            }
            Message::ResetEmulation => {
                if let Some(emulator) = &mut self.emulator {
                    // TODO: This has issues that need to be looked into.
                    emulator.reset();
                }
            }
        }
        Task::none()
    }
}

impl AppModel {
    pub fn about(&self) -> Element<Message> {
        let cosmic_theme::Spacing { space_xxs, .. } = theme::active().cosmic().spacing;

        let icon = widget::svg(widget::svg::Handle::from_memory(APP_ICON));

        let title = widget::text::title3(fl!("app-title"));

        let hash = env!("VERGEN_GIT_SHA");
        let short_hash: String = hash.chars().take(7).collect();
        let date = env!("VERGEN_GIT_COMMIT_DATE");

        let link = widget::button::link(REPOSITORY)
            .on_press(Message::OpenRepositoryUrl)
            .padding(0);

        widget::column()
            .push(icon)
            .push(title)
            .push(link)
            .push(
                widget::button::link(fl!(
                    "git-description",
                    hash = short_hash.as_str(),
                    date = date
                ))
                .on_press(Message::LaunchUrl(format!("{REPOSITORY}/commits/{hash}")))
                .padding(0),
            )
            .align_x(Alignment::Center)
            .spacing(space_xxs)
            .into()
    }

    pub fn update_title(&mut self) -> Task<cosmic::Action<Message>> {
        let mut window_title = fl!("app-title");

        if let Some(Some(rom_name)) = self
            .emulator
            .as_ref()
            .map(|e| e.rom_path().file_name().map(|f| f.to_string_lossy()))
        {
            window_title.push_str(" — ");
            window_title.push_str(&rom_name);
        }

        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(window_title, id)
        } else {
            Task::none()
        }
    }

    fn keymap() -> HashMap<KeyCode, Button> {
        let mut keymap = HashMap::new();
        keymap.insert(KeyCode::KeyX, Button::A);
        keymap.insert(KeyCode::KeyZ, Button::B);
        keymap.insert(KeyCode::Space, Button::Select);
        keymap.insert(KeyCode::Enter, Button::Start);
        keymap.insert(KeyCode::ArrowUp, Button::Up);
        keymap.insert(KeyCode::ArrowDown, Button::Down);
        keymap.insert(KeyCode::ArrowLeft, Button::Left);
        keymap.insert(KeyCode::ArrowRight, Button::Right);
        keymap
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    About,
    OpenFile,
    ToggleEmulation,
    ResetEmulation,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::About => Message::ToggleContextPage(ContextPage::About),
            MenuAction::OpenFile => Message::OpenFileDialog,
            MenuAction::ToggleEmulation => Message::ToggleEmulation,
            MenuAction::ResetEmulation => Message::ResetEmulation,
        }
    }
}
