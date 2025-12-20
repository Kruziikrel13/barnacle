use std::{path::PathBuf, sync::Arc};

use barnacle_lib::{
    Repository,
    repository::{Game, Profile},
};
use iced::{
    Color, Element,
    Length::{self, Fill},
    Task, Theme, application,
    widget::{
        button, center, column, combo_box, container, mouse_area, opaque, row, space, stack, text,
    },
};
use parking_lot::RwLock;
use tokio::task::spawn_blocking;
use tracing::Level;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use crate::{
    components::{
        add_mod_dialog::{self, AddModDialog},
        library_manager::{self, LibraryManager},
        mod_list::{self, ModList},
    },
    config::GuiConfig,
    icons::icon,
};

pub mod components;
pub mod config;
pub mod icons;

fn main() -> iced::Result {
    // Human friendly panicking in release mode
    human_panic::setup_panic!();

    // Logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    application(App::new, App::update, App::view)
        .theme(App::theme)
        .title(App::title)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    AddModButtonPressed,
    LibraryManagerButtonPressed,
    ModAdded,
    GameSelected(Game),
    ProfileSelected(Profile),
    // Components
    AddModDialog(add_mod_dialog::Message),
    ModList(mod_list::Message),
    LibraryManager(library_manager::Message),
}

struct App {
    repo: Repository,
    title: String,
    theme: Theme,
    game_selector: GameSelector,
    profile_selector: ProfileSelector,
    show_add_mod_dialog: bool,
    show_library_manager: bool,
    // Components
    add_mod_dialog: AddModDialog,
    mod_list: ModList,
    library_manager: LibraryManager,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let repo = Repository::new();
        let cfg = Arc::new(RwLock::new(GuiConfig::load()));
        let theme = cfg.read().theme();

        // if repo.games().unwrap().is_empty() {
        //     let mut game = repo
        //         .add_game(
        //             "Skyrim",
        //             barnacle_lib::repository::DeployKind::CreationEngine,
        //         )
        //         .unwrap();
        //     let profile = game.add_profile("Test Profile").unwrap();
        //
        //     repo.set_current_profile(&profile).unwrap();
        //
        //     for i in 1..100 {
        //         let mod_ = game.add_mod(format!("Mod{}", i).as_str(), None).unwrap();
        //         profile.add_mod_entry(mod_).unwrap();
        //     }
        // }

        let current_profile = repo.current_profile().unwrap();
        let current_game = repo.current_game().unwrap();

        let game_options = repo.games().unwrap();

        let profile_options = if let Some(game) = &current_game {
            game.profiles().unwrap()
        } else {
            Vec::new()
        };

        let (add_mod_dialog, _add_mod_dialog_class) = AddModDialog::new(repo.clone());
        let (mod_list, mod_list_task) = ModList::new(repo.clone(), cfg.clone());
        let (library_manager, library_manager_task) = LibraryManager::new(repo.clone());

        (
            Self {
                repo,
                title: "Barnacle".into(),
                theme,
                game_selector: GameSelector {
                    state: combo_box::State::new(game_options),
                    selected: current_game,
                },
                profile_selector: ProfileSelector {
                    state: combo_box::State::new(profile_options),
                    selected: current_profile,
                },
                show_add_mod_dialog: false,
                show_library_manager: false,
                add_mod_dialog,
                mod_list,
                library_manager,
            },
            Task::batch([
                mod_list_task.map(Message::ModList),
                library_manager_task.map(Message::LibraryManager),
            ]),
        )
    }

    // Update application state based on messages passed by view()
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // Redirect messages to relevant child components
            Message::AddModDialog(message) => match self.add_mod_dialog.update(message) {
                add_mod_dialog::Action::None => Task::none(),
                add_mod_dialog::Action::Run(task) => task.map(Message::AddModDialog),
                add_mod_dialog::Action::Cancel => {
                    self.show_add_mod_dialog = false;
                    Task::none()
                }
                add_mod_dialog::Action::AddMod { name, path } => {
                    self.show_add_mod_dialog = false;
                    let repo = self.repo.clone();
                    Task::perform(
                        async {
                            spawn_blocking(move || {
                                // TODO: Should this just silenty fail? I guess the "Add Mod" button
                                // won't even be enabled if there isn't a current profile but still
                                // doesn't feel right.
                                if let Some(profile) = repo.current_profile().unwrap() {
                                    let game = profile.parent().unwrap();

                                    let mod_ =
                                        game.add_mod(&name, Some(&PathBuf::from(path))).unwrap();
                                    profile.add_mod_entry(mod_).unwrap();
                                }
                            })
                            .await
                        },
                        |_| Message::ModAdded,
                    )
                }
            },
            Message::ModList(message) => self.mod_list.update(message).map(Message::ModList),
            Message::LibraryManager(message) => match self.library_manager.update(message) {
                library_manager::Action::None => Task::none(),
                library_manager::Action::Run(task) => task.map(Message::LibraryManager),
                library_manager::Action::Close => {
                    self.show_library_manager = false;
                    Task::none()
                }
            },
            Message::AddModButtonPressed => {
                self.show_add_mod_dialog = true;
                Task::none()
            }
            Message::LibraryManagerButtonPressed => {
                self.show_library_manager = true;
                Task::none()
            }
            Message::ModAdded => self.mod_list.refresh().map(Message::ModList),
            Message::GameSelected(game) => {
                self.game_selector.selected = Some(game);
                Task::none()
            }
            Message::ProfileSelected(profile) => {
                self.profile_selector.selected = Some(profile);
                Task::none()
            }
        }
    }

    // Render the application and pass along messages from components to update()
    pub fn view(&self) -> Element<'_, Message> {
        let content = column![
            // Top bar
            row![
                text("Game:"),
                combo_box(
                    &self.game_selector.state,
                    "...",
                    self.game_selector.selected.as_ref(),
                    Message::GameSelected
                ),
                button(icon("play")),
                text("Profile:"),
                combo_box(
                    &self.profile_selector.state,
                    "...",
                    self.profile_selector.selected.as_ref(),
                    Message::ProfileSelected
                ),
                space::horizontal(),
                button(icon("library")).on_press(Message::LibraryManagerButtonPressed),
                button(icon("settings")),
                button(icon("notifications"))
            ],
            // Action bar
            row![button("Add Mod").on_press(Message::AddModButtonPressed)],
            // Mod list
            self.mod_list.view().map(Message::ModList),
        ]
        .height(Fill);

        if self.show_library_manager {
            modal(
                content,
                self.library_manager.view().map(Message::LibraryManager),
                None,
            )
        } else if self.show_add_mod_dialog {
            modal(
                content,
                self.add_mod_dialog.view().map(Message::AddModDialog),
                None,
            )
        } else {
            content.into()
        }
    }

    pub fn title(&self) -> String {
        self.title.clone()
    }

    pub fn theme(&self) -> Theme {
        self.theme.clone()
    }
}

struct GameSelector {
    state: combo_box::State<Game>,
    selected: Option<Game>,
}

struct ProfileSelector {
    state: combo_box::State<Profile>,
    selected: Option<Profile>,
}

pub fn modal<'a, Message>(
    base: impl Into<Element<'a, Message>>,
    content: impl Into<Element<'a, Message>>,
    on_click_outside: Option<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let mouse_area = mouse_area(center(opaque(content)).style(|_theme| {
        container::Style {
            background: Some(
                Color {
                    a: 0.8,
                    ..Color::BLACK
                }
                .into(),
            ),
            ..container::Style::default()
        }
    }));

    stack![
        base.into(),
        opaque(if let Some(msg) = on_click_outside {
            mouse_area.on_press(msg)
        } else {
            mouse_area
        })
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
