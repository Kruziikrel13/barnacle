use crate::icons::icon;
use barnacle_lib::{Repository, repository::Game};
use iced::{
    Element, Length, Task,
    widget::{Column, button, column, container, row, scrollable, space, text},
};

use crate::{
    components::library_manager::{
        TAB_PADDING,
        games_tab::{edit_dialog::EditDialog, new_dialog::NewDialog},
    },
    modal,
};

mod edit_dialog;
mod new_dialog;

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Vec<Game>),
    ShowNewDialog,
    ShowEditDialog(Game),
    DeleteButtonPressed(Game),
    // Child messages
    NewDialog(new_dialog::Message),
    EditDialog(edit_dialog::Message),
}

#[derive(Debug)]
pub enum Action {
    None,
    Run(Task<Message>),
    DeleteGame(Game),
}

pub enum State {
    Loading,
    Error(String),
    Loaded(Vec<Game>),
}

pub struct Tab {
    repo: Repository,
    state: State,
    show_new_dialog: bool,
    show_edit_dialog: bool,
    // Components
    new_dialog: NewDialog,
    edit_dialog: EditDialog,
}

impl Tab {
    pub fn new(repo: Repository) -> (Self, Task<Message>) {
        let (new_dialog, _) = NewDialog::new(repo.clone());
        let (edit_dialog, _) = EditDialog::new();

        (
            Self {
                repo: repo.clone(),
                state: State::Loading,
                show_new_dialog: false,
                show_edit_dialog: false,
                new_dialog,
                edit_dialog,
            },
            Task::perform(
                {
                    let repo = repo.clone();
                    async move { repo.games().unwrap() }
                },
                Message::Loaded,
            ),
        )
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            // State
            Message::Loaded(games) => {
                self.state = State::Loaded(games);
                Action::None
            }
            Message::ShowNewDialog => {
                self.show_new_dialog = true;
                Action::None
            }
            Message::ShowEditDialog(game) => {
                self.edit_dialog.load(game);
                self.show_edit_dialog = true;
                Action::None
            }
            Message::DeleteButtonPressed(game) => {
                // TODO: Remove once I can get a StaleHandle error from missing ID
                self.state = State::Loading;
                Action::DeleteGame(game)
            }
            // Components
            Message::NewDialog(msg) => match msg {
                new_dialog::Message::CancelPressed => {
                    self.show_new_dialog = false;
                    self.new_dialog.clear();
                    Action::None
                }
                new_dialog::Message::GameCreated => {
                    self.state = State::Loading;
                    self.show_new_dialog = false;
                    Action::Run(self.refresh_list())
                }
                _ => Action::Run(self.new_dialog.update(msg).map(Message::NewDialog)),
            },
            Message::EditDialog(msg) => match msg {
                edit_dialog::Message::CancelPressed => {
                    self.show_edit_dialog = false;
                    Action::None
                }
                edit_dialog::Message::GameEdited => {
                    self.show_edit_dialog = false;
                    Action::None
                }
                _ => Action::Run(self.edit_dialog.update(msg).map(Message::EditDialog)),
            },
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match &self.state {
            State::Loading => column![text("Loading...")].into(),
            State::Error(_e) => column![text("ERROR!")].into(),
            State::Loaded(games) => {
                let children = games.iter().map(game_row);

                let content = column![
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
            }
        }
    }

    pub fn refresh_list(&self) -> Task<Message> {
        Task::perform(
            {
                let repo = self.repo.clone();
                async move { repo.games().unwrap() }
            },
            Message::Loaded,
        )
    }
}

fn game_row<'a>(game: &Game) -> Element<'a, Message> {
    container(
        row![
            text(game.name().unwrap()),
            space::horizontal(),
            button(icon("edit")).on_press(Message::ShowEditDialog(game.clone())),
            button(icon("delete")).on_press(Message::DeleteButtonPressed(game.clone()))
        ]
        .padding(12),
    )
    .width(Length::Fill)
    .style(container::bordered_box)
    .into()
}
