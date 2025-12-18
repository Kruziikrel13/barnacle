use crate::icons::icon;
use barnacle_lib::{
    Repository,
    repository::{Game, Profile},
};
use iced::{
    Element, Length, Task,
    widget::{Column, button, column, combo_box, container, row, scrollable, space, text},
};

use crate::{
    components::library_manager::{
        TAB_PADDING,
        profiles_tab::{edit_dialog::EditDialog, new_dialog::NewDialog},
    },
    modal,
};

mod edit_dialog;
mod new_dialog;

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Vec<Profile>),
    ProfileDeleted,
    ShowNewDialog,
    ShowEditDialog(Profile),
    DeleteButtonPressed(Profile),
    GameSelected(Game),
    // Child messages
    NewDialog(new_dialog::Message),
    EditDialog(edit_dialog::Message),
}

pub enum Action {
    None,
    Run(Task<Message>),
}

pub enum State {
    Loading,
    Error(String),
    Loaded(Vec<Profile>),
}

pub struct Tab {
    state: State,
    selected_game: Option<Game>,
    game_options: combo_box::State<Game>,
    show_new_dialog: bool,
    show_edit_dialog: bool,
    // Components
    new_dialog: NewDialog,
    edit_dialog: EditDialog,
}

impl Tab {
    pub fn new(repo: Repository) -> (Self, Task<Message>) {
        let games = repo.games().unwrap();
        let selected_game = repo.current_game().unwrap();

        let (new_dialog, _) = NewDialog::new();
        let (edit_dialog, _) = EditDialog::new();

        let task = match &selected_game {
            Some(game) => Task::perform(
                {
                    let game = game.clone();
                    async move { game.profiles().unwrap() }
                },
                Message::Loaded,
            ),
            None => Task::none(),
        };

        (
            Self {
                selected_game: selected_game.clone(),
                game_options: combo_box::State::new(games),
                state: State::Loading,
                show_new_dialog: false,
                show_edit_dialog: false,
                new_dialog,
                edit_dialog,
            },
            task,
        )
    }

    fn refresh(&self) -> Task<Message> {
        if let Some(selected_game) = &self.selected_game {
            Task::perform(
                {
                    let game = selected_game.clone();
                    async move { game.profiles().unwrap() }
                },
                Message::Loaded,
            )
        } else {
            Task::none()
        }
    }

    pub fn update(&mut self, message: Message) -> Action {
        if let Some(selected_game) = &self.selected_game {
            match message {
                // State
                Message::Loaded(profiles) => {
                    self.state = State::Loaded(profiles);
                    Action::None
                }
                Message::ProfileDeleted => Action::Run(self.refresh()),
                // Components
                Message::ShowNewDialog => {
                    self.show_new_dialog = true;
                    Action::None
                }
                Message::ShowEditDialog(profile) => {
                    self.edit_dialog.load(profile);
                    self.show_edit_dialog = true;
                    Action::None
                }
                Message::DeleteButtonPressed(profile) => Action::Run(Task::perform(
                    {
                        // So we don't try to query deleted profiles
                        self.state = State::Loading;

                        let mut game = selected_game.clone();
                        async move { game.remove_profile(profile).unwrap() }
                    },
                    |_| Message::ProfileDeleted,
                )),
                Message::GameSelected(game) => {
                    self.selected_game = Some(game.clone());
                    self.new_dialog.load(game.clone());
                    Action::Run(self.refresh())
                }
                Message::NewDialog(msg) => match msg {
                    new_dialog::Message::CancelPressed => {
                        self.show_new_dialog = false;
                        self.new_dialog.clear();
                        Action::None
                    }
                    new_dialog::Message::ProfileCreated => {
                        self.state = State::Loading;
                        self.show_new_dialog = false;
                        Action::Run(self.refresh())
                    }
                    _ => Action::Run(self.new_dialog.update(msg).map(Message::NewDialog)),
                },
                Message::EditDialog(msg) => match msg {
                    edit_dialog::Message::CancelPressed => {
                        self.show_edit_dialog = false;
                        Action::None
                    }
                    edit_dialog::Message::ProfileEdited => {
                        self.show_edit_dialog = false;
                        Action::None
                    }
                    _ => Action::Run(self.edit_dialog.update(msg).map(Message::EditDialog)),
                },
            }
        } else {
            Action::None
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match &self.state {
            State::Loading => column![text("Loading...")].into(),
            State::Error(_e) => column![text("ERROR!")].into(),
            State::Loaded(profiles) => {
                if let Some(selected_game) = &self.selected_game {
                    let children = profiles.iter().map(profile_row);

                    let content = column![
                        combo_box(
                            &self.game_options,
                            "Select a game...",
                            Some(selected_game),
                            Message::GameSelected
                        ),
                        row![button("New").on_press(Message::ShowNewDialog)],
                        scrollable(Column::with_children(children)).width(Length::Fill)
                    ]
                    .padding(TAB_PADDING);

                    if self.show_new_dialog {
                        modal(
                            content,
                            self.new_dialog.view().map(Message::NewDialog),
                            None,
                        )
                    } else if self.show_edit_dialog {
                        modal(
                            content,
                            self.edit_dialog.view().map(Message::EditDialog),
                            None,
                        )
                    } else {
                        content.into()
                    }
                } else {
                    text("No games found").into()
                }
            }
        }
    }
}

fn profile_row<'a>(profile: &Profile) -> Element<'a, Message> {
    container(
        row![
            text(profile.name().unwrap()),
            space::horizontal(),
            button(icon("edit")).on_press(Message::ShowEditDialog(profile.clone())),
            button(icon("delete")).on_press(Message::DeleteButtonPressed(profile.clone()))
        ]
        .padding(12),
    )
    .width(Length::Fill)
    .style(container::bordered_box)
    .into()
}
