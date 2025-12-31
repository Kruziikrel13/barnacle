use crate::{components::library_manager::profiles_tab::new_dialog::NewProfile, icons::icon};
use adisruption_widgets::generic_overlay::overlay_button;
use barnacle_lib::{
    Repository,
    repository::{Game, Profile},
};
use iced::{
    Element, Length, Task,
    widget::{Column, button, column, container, row, scrollable, space, text},
};
use tokio::task::spawn_blocking;

use crate::components::library_manager::profiles_tab::{
    edit_dialog::EditDialog, new_dialog::NewDialog,
};

pub mod edit_dialog;
pub mod new_dialog;

#[derive(Debug, Clone)]
pub enum Message {
    StateChanged(State),
    ProfileDeleted,
    LoadEditDialog(Profile),
    DeleteButtonPressed(Profile),
    ProfileCreated,
    ProfileEdited,
    // Child messages
    NewDialog(new_dialog::Message),
    EditDialog(edit_dialog::Message),
}

pub enum Action {
    None,
    Run(Task<Message>),
    Refresh,
    Create(NewProfile),
}

#[derive(Debug, Clone)]
pub enum State {
    Loading,
    Error(String),
    Loaded(Vec<Profile>),
}

pub struct Tab {
    repo: Repository,
    state: State,

    // Children
    new_dialog: NewDialog,
    edit_dialog: EditDialog,
}

impl Tab {
    pub fn new(repo: Repository) -> Self {
        let (new_dialog, _) = NewDialog::new();
        let (edit_dialog, _) = EditDialog::new();

        Self {
            repo: repo.clone(),
            state: State::Loading,

            // Widget state
            new_dialog,
            edit_dialog,
        }
    }

    pub fn refresh(&self, game: &Game) -> Task<Message> {
        let game = game.clone();
        Task::perform(
            {
                async {
                    spawn_blocking(move || State::Loaded(game.profiles().unwrap()))
                        .await
                        .unwrap()
                }
            },
            Message::StateChanged,
        )
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::StateChanged(state) => {
                self.state = state;
                Action::None
            }
            Message::ProfileDeleted => {
                self.state = State::Loading;
                Action::Refresh
            }
            Message::ProfileCreated => Action::Refresh,
            Message::ProfileEdited => Action::Refresh,
            Message::DeleteButtonPressed(profile) => {
                self.state = State::Loading;

                Action::Run(Task::perform(
                    async {
                        spawn_blocking(move || {
                            profile.remove().unwrap();
                        })
                        .await
                        .unwrap()
                    },
                    |_| Message::ProfileDeleted,
                ))
            }
            Message::LoadEditDialog(profile) => {
                self.edit_dialog.load(profile);
                Action::None
            }
            Message::NewDialog(message) => match self.new_dialog.update(message) {
                new_dialog::Action::None => Action::None,
                new_dialog::Action::Run(task) => Action::Run(task.map(Message::NewDialog)),
                new_dialog::Action::Create(new_profile) => {
                    self.state = State::Loading;
                    Action::Create(new_profile)
                }
            },
            Message::EditDialog(message) => match &self.state {
                State::Loaded { .. } => match self.edit_dialog.update(message) {
                    edit_dialog::Action::None => Action::None,
                    edit_dialog::Action::Run(task) => Action::Run(task.map(Message::EditDialog)),
                    edit_dialog::Action::Cancel => Action::None,
                    edit_dialog::Action::Edit { profile, name } => Action::Run(Task::perform(
                        async {
                            spawn_blocking(move || {
                                profile.set_name(&name).unwrap();
                            })
                            .await
                        },
                        |_| Message::ProfileEdited,
                    )),
                },
                _ => Action::None,
            },
        }
    }
    pub fn view(&self) -> Element<'_, Message> {
        match &self.state {
            State::Loading => column![text("Loading...")].into(),
            State::Error(e) => column![text(e)].into(),
            State::Loaded(profiles) => column![
                row![
                    overlay_button(
                        "New",
                        "Add Profile",
                        self.new_dialog.view().map(Message::NewDialog)
                    )
                    .overlay_width_dynamic(|window_width| Length::Fixed(window_width * 0.4))
                    .overlay_height_dynamic(|window_height| Length::Fixed(window_height * 0.6))
                    .hide_header()
                    .opaque(true)
                    .id(new_dialog::ID)
                ],
                scrollable(Column::with_children(
                    profiles.iter().map(|p| self.profile_row(p))
                ))
            ]
            .into(),
        }
    }

    fn profile_row<'a>(&'a self, profile: &'a Profile) -> Element<'a, Message> {
        container(
            row![
                text(profile.name().unwrap()),
                space::horizontal(),
                overlay_button(
                    icon("edit"),
                    "Edit Profile",
                    self.edit_dialog.view().map(Message::EditDialog)
                )
                .on_open(|_, _| Message::LoadEditDialog(profile.clone()))
                .overlay_width_dynamic(|window_width| Length::Fixed(window_width * 0.4))
                .overlay_height_dynamic(|window_height| Length::Fixed(window_height * 0.6))
                .hide_header()
                .opaque(true)
                .id("edit_profile_dialog"),
                button(icon("delete")).on_press(Message::DeleteButtonPressed(profile.clone()))
            ]
            .padding(12),
        )
        .width(Length::Fill)
        .style(container::bordered_box)
        .into()
    }
}
