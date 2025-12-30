use crate::icons::icon;
use adisruption_widgets::generic_overlay::overlay_button;
use barnacle_lib::{
    Repository,
    repository::{Game, Profile},
};
use iced::{
    Element, Length, Task,
    widget::{Column, button, column, combo_box, container, row, scrollable, space, text},
};
use tokio::task::spawn_blocking;

use crate::components::library_manager::profiles_tab::{
    edit_dialog::EditDialog, new_dialog::NewDialog,
};

mod edit_dialog;
mod new_dialog;

#[derive(Debug, Clone)]
pub enum Message {
    StateLoaded(State),
    ProfileDeleted,
    LoadEditDialog(Profile),
    DeleteButtonPressed(Profile),
    GameSelected(Game),
    ProfileCreated,
    ProfileEdited,
    // Child messages
    NewDialog(new_dialog::Message),
    EditDialog(edit_dialog::Message),
}

pub enum Action {
    None,
    Run(Task<Message>),
}

#[derive(Debug, Clone)]
pub enum State {
    Loading,
    Error(String),
    NoGames,
    Loaded {
        selected_game: Game,
        games: Vec<Game>,
        profiles: Vec<Profile>,
    },
}

pub struct Tab {
    repo: Repository,
    state: State,

    // Widget state
    game_options: combo_box::State<Game>,

    // Children
    new_dialog: NewDialog,
    edit_dialog: EditDialog,
}

impl Tab {
    pub fn new(repo: Repository) -> (Self, Task<Message>) {
        let (new_dialog, _) = NewDialog::new();
        let (edit_dialog, _) = EditDialog::new();

        (
            Self {
                repo: repo.clone(),
                state: State::Loading,

                // Widget state
                game_options: combo_box::State::new(Vec::new()),
                new_dialog,
                edit_dialog,
            },
            load_state(repo),
        )
    }

    pub fn refresh(&self) -> Task<Message> {
        load_state(self.repo.clone())
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::StateLoaded(state) => {
                // Update widget state
                match &state {
                    State::Loaded { games, .. } => {
                        self.game_options = combo_box::State::new(games.clone());
                    }
                    State::NoGames => {
                        self.game_options = combo_box::State::new(Vec::new());
                    }
                    _ => {}
                }

                self.state = state;
                Action::None
            }
            Message::ProfileDeleted => {
                self.state = State::Loading;
                Action::Run(self.refresh())
            }
            Message::GameSelected(game) => {
                self.new_dialog.load(game.clone());
                Action::Run(self.refresh())
            }
            Message::ProfileCreated => Action::Run(self.refresh()),
            Message::ProfileEdited => Action::Run(self.refresh()),
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
            Message::NewDialog(message) => match &self.state {
                State::Loaded { selected_game, .. } => match self.new_dialog.update(message) {
                    new_dialog::Action::None => Action::None,
                    new_dialog::Action::Run(task) => Action::Run(task.map(Message::NewDialog)),
                    new_dialog::Action::Cancel => {
                        self.new_dialog.clear();
                        Action::None
                    }
                    new_dialog::Action::Create { name } => {
                        let selected_game = selected_game.clone();

                        self.state = State::Loading;
                        Action::Run(Task::perform(
                            async {
                                spawn_blocking(move || {
                                    selected_game.add_profile(&name).unwrap();
                                })
                                .await
                            },
                            |_| Message::ProfileCreated,
                        ))
                    }
                },
                _ => Action::None,
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
            State::NoGames => text("No games found").into(),
            State::Loaded {
                selected_game,
                profiles,
                ..
            } => column![
                combo_box(
                    &self.game_options,
                    "Select a game...",
                    Some(selected_game),
                    Message::GameSelected
                ),
                row![overlay_button(
                    "New",
                    "Add Profile",
                    self.new_dialog.view().map(Message::NewDialog)
                )],
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
                .on_open(|_, _| Message::LoadEditDialog(profile.clone())),
                button(icon("delete")).on_press(Message::DeleteButtonPressed(profile.clone()))
            ]
            .padding(12),
        )
        .width(Length::Fill)
        .style(container::bordered_box)
        .into()
    }
}

pub fn load_state(repo: Repository) -> Task<Message> {
    Task::perform(
        async {
            spawn_blocking(move || {
                let games = repo.games().unwrap();
                if games.is_empty() {
                    return State::NoGames;
                }

                let selected_game = match repo.active_game().unwrap() {
                    Some(game) => game,
                    None => games.first().cloned().unwrap(),
                };
                let profiles = selected_game.profiles().unwrap();

                State::Loaded {
                    selected_game,
                    games,
                    profiles,
                }
            })
            .await
            .unwrap()
        },
        Message::StateLoaded,
    )
}
