use std::{path::PathBuf, sync::Arc};

use adisruption_widgets::generic_overlay::{self, ResizeMode, overlay_button};
use barnacle_lib::{Repository, repository::Profile};
use iced::{
    Element,
    Length::{self, Fill},
    Task, Theme,
    advanced::widget::operate,
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
};

pub mod add_mod_dialog;
pub mod library_manager;
pub mod mod_list;

#[derive(Debug, Clone)]
pub enum Message {
    AddModButtonPressed,
    LibraryManagerButtonPressed,
    ModAdded,
    ProfileSelected(Profile),
    GameAdded,
    ProfileAdded,
    GameEdited,
    GameDeleted,
    // Components
    AddModDialog(add_mod_dialog::Message),
    ModList(mod_list::Message),
    LibraryManager(library_manager::Message),
}

pub struct App {
    repo: Repository,
    title: String,
    theme: Theme,
    profile_selector: ProfileSelector,
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

        let active_profile = repo.active_profile().unwrap();
        let active_game = repo.active_game().unwrap();

        let profile_options = if let Some(game) = &active_game {
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
                profile_selector: ProfileSelector {
                    state: combo_box::State::new(profile_options),
                    selected: active_profile,
                },
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

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AddModDialog(message) => match self.add_mod_dialog.update(message) {
                add_mod_dialog::Action::None => Task::none(),
                add_mod_dialog::Action::Run(task) => task.map(Message::AddModDialog),
                add_mod_dialog::Action::AddMod { name, path } => {
                    let repo = self.repo.clone();
                    Task::batch([
                        Task::perform(
                            async {
                                spawn_blocking(move || {
                                    // TODO: Should this just silenty fail? I guess the "Add Mod" button
                                    // won't even be enabled if there isn't a current profile but still
                                    // doesn't feel right.
                                    if let Some(profile) = repo.active_profile().unwrap() {
                                        let game = profile.parent().unwrap();

                                        let mod_ = game
                                            .add_mod(&name, Some(&PathBuf::from(path)))
                                            .unwrap();
                                        profile.add_mod_entry(mod_).unwrap();
                                    }
                                })
                                .await
                            },
                            |_| Message::ModAdded,
                        ),
                        operate(generic_overlay::close::<Message>(add_mod_dialog::ID.into())),
                    ])
                }
            },
            Message::ModList(message) => self.mod_list.update(message).map(Message::ModList),
            Message::LibraryManager(message) => match self.library_manager.update(message) {
                library_manager::Action::None => Task::none(),
                library_manager::Action::Run(task) => task.map(Message::LibraryManager),
                library_manager::Action::CreateGame(new_game) => Task::batch([
                    Task::perform({
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
                    operate(generic_overlay::close::<Message>(library_manager::new_game_dialog::ID.into())),
                ]),
                library_manager::Action::CreateProfile { game, new_profile } => Task::batch([
                    Task::perform({
                        let game = game.clone();
                        async {
                            spawn_blocking(move || game.add_profile(&new_profile.name).unwrap())
                                .await
                                .unwrap()
                        }
                    },
                    |_| Message::ProfileAdded,
                    ),
                    operate(generic_overlay::close::<Message>(library_manager::profiles_tab::new_dialog::ID.into())),
                ]),
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
                library_manager::Action::Close => Task::none(),
            },
            Message::AddModButtonPressed => Task::none(),
            Message::LibraryManagerButtonPressed => Task::none(),
            Message::ModAdded => self.mod_list.refresh().map(Message::ModList),
            Message::ProfileSelected(profile) => {
                self.profile_selector.selected = Some(profile);
                Task::none()
            }
            Message::GameAdded
            | Message::GameEdited
            | Message::GameDeleted
            // TODO: Update profile selector
            | Message::ProfileAdded => self.library_manager.refresh().map(Message::LibraryManager),
        }
    }

    // Render the application and pass along messages from components to update()
    pub fn view(&self) -> Element<'_, Message> {
        column![
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
                overlay_button(
                    icon("library"),
                    "Library Manager",
                    self.library_manager.view().map(Message::LibraryManager)
                )
                .overlay_width_dynamic(|window_width| Length::Fixed(window_width * 0.8))
                .overlay_height_dynamic(|window_height| Length::Fixed(window_height * 0.8))
                .resizable(ResizeMode::Always),
                button(icon("settings")),
                button(icon("notifications"))
            ],
            // Action bar
            row![
                overlay_button(
                    "Add Mod",
                    "Add Mod",
                    self.add_mod_dialog.view().map(Message::AddModDialog)
                )
                .overlay_width_dynamic(|window_width| Length::Fixed(window_width * 0.5))
                .overlay_height_dynamic(|window_height| Length::Fixed(window_height * 0.6))
                .hide_header()
                .opaque(true)
                .id(add_mod_dialog::ID)
            ],
            // Mod list
            self.mod_list.view().map(Message::ModList),
        ]
        .height(Fill)
        .into()
    }

    pub fn title(&self) -> String {
        self.title.clone()
    }

    pub fn theme(&self) -> Theme {
        self.theme.clone()
    }
}

struct ProfileSelector {
    state: combo_box::State<Profile>,
    selected: Option<Profile>,
}
