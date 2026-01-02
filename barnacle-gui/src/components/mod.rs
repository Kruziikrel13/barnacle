use std::{path::PathBuf, sync::Arc};

use barnacle_lib::{Repository, repository::Profile};
use iced::{
    Element,
    Length::Fill,
    Task, Theme,
    widget::{button, column, combo_box, row, space, text},
};
use parking_lot::RwLock;
use tokio::task::spawn_blocking;

use crate::{
    components::{
        add_mod_dialog::AddModDialog, library_manager::LibraryManager, mod_list::ModList,
    },
    config::GuiConfig,
    icons::icon,
    modal,
};

pub mod add_mod_dialog;
pub mod library_manager;
pub mod mod_list;

#[derive(Debug, Clone)]
pub enum Message {
    StateChanged(State),
    AddModButtonPressed,
    LibraryManagerButtonPressed,
    ModAdded,
    ProfileSelected(Profile),
    GameAdded,
    ProfileAdded,
    ProfileActivated(Profile),
    GameEdited,
    GameDeleted,
    // Components
    AddModDialog(add_mod_dialog::Message),
    ModList(mod_list::Message),
    LibraryManager(library_manager::Message),
}

#[derive(Debug, Clone)]
pub enum State {
    Loading,
    Error(String),
    NoGames,
    Loaded {
        active_profile: Option<Profile>,
        profiles: Vec<Profile>,
    },
}

pub struct App {
    repo: Repository,
    state: State,
    title: String,
    theme: Theme,
    profile_selector: ProfileSelector,
    // State
    show_library_manager: bool,
    show_add_mod_dialog: bool,
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

        let (add_mod_dialog, _add_mod_dialog_class) = AddModDialog::new(repo.clone());
        let mod_list = ModList::new(repo.clone(), cfg.clone());
        let (library_manager, library_manager_task) = LibraryManager::new(repo.clone());

        (
            Self {
                repo: repo.clone(),
                state: State::Loading,
                title: "Barnacle".into(),
                theme,
                show_library_manager: false,
                show_add_mod_dialog: false,
                profile_selector: ProfileSelector {
                    state: combo_box::State::new(Vec::new()),
                    selected: None,
                },
                add_mod_dialog,
                mod_list,
                library_manager,
            },
            Task::batch([
                library_manager_task.map(Message::LibraryManager),
                load_state(repo.clone()),
            ]),
        )
    }

    pub fn refresh(&self) -> Task<Message> {
        load_state(self.repo.clone())
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::StateChanged(state) => {
                self.state = state;

                if let State::Loaded {
                    active_profile,
                    profiles,
                } = &self.state
                {
                    self.profile_selector.state = combo_box::State::new(profiles.clone());
                    self.profile_selector.selected = active_profile.clone();

                    if let Some(active_profile) = active_profile {
                        return self.mod_list.refresh(active_profile).map(Message::ModList);
                    }
                }

                Task::none()
            }
            Message::AddModDialog(message) => match self.add_mod_dialog.update(message) {
                add_mod_dialog::Action::None => Task::none(),
                add_mod_dialog::Action::Run(task) => task.map(Message::AddModDialog),
                add_mod_dialog::Action::AddMod { name, path } => {
                    self.show_add_mod_dialog = false;
                    let repo = self.repo.clone();
                    Task::perform(
                        async {
                            spawn_blocking(move || {
                                if let Some(profile) = repo.active_profile().unwrap() {
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
                add_mod_dialog::Action::Cancel => {
                    self.show_add_mod_dialog = false;
                    Task::none()
                }
            },
            Message::ModList(message) => match self.mod_list.update(message) {
                mod_list::Action::None => Task::none(),
                mod_list::Action::Run(task) => task.map(Message::ModList),
            },
            Message::LibraryManager(message) => match self.library_manager.update(message) {
                library_manager::Action::None => Task::none(),
                library_manager::Action::Run(task) => task.map(Message::LibraryManager),
                library_manager::Action::CreateGame(new_game) => Task::perform(
                    {
                        let repo = self.repo.clone();
                        async move {
                            spawn_blocking(move || {
                                repo.add_game(&new_game.name, new_game.deploy_kind)
                            })
                            .await
                        }
                    },
                    |_| Message::GameAdded,
                ),
                library_manager::Action::CreateProfile { game, new_profile } => Task::perform(
                    {
                        let game = game.clone();
                        async {
                            spawn_blocking(move || game.add_profile(&new_profile.name).unwrap())
                                .await
                                .unwrap()
                        }
                    },
                    |_| Message::ProfileAdded,
                ),
                // library_manager::Action::EditGame(edit) => Task::perform(
                //     async move {
                //         spawn_blocking(move || {
                //             edit.game.set_name(&edit.name).unwrap();
                //             edit.game.set_deploy_kind(edit.deploy_kind).unwrap();
                //         })
                //         .await
                //         .unwrap()
                //     },
                //     |_| Message::GameEdited,
                // ),
                library_manager::Action::DeleteGame(game) => Task::perform(
                    async move { spawn_blocking(move || game.remove().unwrap()).await },
                    |_| Message::GameDeleted,
                ),
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
            Message::ModAdded => {
                if let Some(active_profile) = &self.profile_selector.selected {
                    self.mod_list.refresh(active_profile).map(Message::ModList)
                } else {
                    Task::none()
                }
            }
            Message::ProfileSelected(profile) => {
                self.profile_selector.selected = Some(profile.clone());
                Task::perform(
                    async {
                        spawn_blocking(move || {
                            profile.make_active().unwrap();
                            profile
                        })
                        .await
                        .unwrap()
                    },
                    Message::ProfileActivated,
                )
            }
            Message::ProfileAdded => Task::batch([
                self.refresh(),
                self.library_manager.refresh().map(Message::LibraryManager),
            ]),
            Message::ProfileActivated(profile) => Task::batch([
                self.refresh(),
                self.mod_list.refresh(&profile).map(Message::ModList),
            ]),
            Message::GameAdded | Message::GameEdited | Message::GameDeleted => {
                self.library_manager.refresh().map(Message::LibraryManager)
            }
        }
    }

    // Render the application and pass along messages from components to update()
    pub fn view(&self) -> Element<'_, Message> {
        let content = column![
            // Top bar
            row![
                button("Launch game"),
                button(icon("wrench")),
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
            row![
                button("Add Mod").on_press_maybe(
                    self.profile_selector
                        .selected
                        .is_some()
                        .then_some(Message::AddModButtonPressed)
                )
            ],
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

fn load_state(repo: Repository) -> Task<Message> {
    let repo = repo.clone();
    Task::perform(
        async {
            spawn_blocking(move || {
                if let Some(active_game) = repo.active_game().unwrap() {
                    State::Loaded {
                        active_profile: repo.active_profile().unwrap(),
                        profiles: active_game.profiles().unwrap(),
                    }
                } else {
                    State::NoGames
                }
            })
            .await
            .unwrap()
        },
        Message::StateChanged,
    )
}

#[derive(Debug)]
struct ProfileSelector {
    state: combo_box::State<Profile>,
    selected: Option<Profile>,
}
