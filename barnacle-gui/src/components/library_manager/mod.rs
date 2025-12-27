use crate::{components::library_manager::new_game_dialog::NewGame, icons::icon, modal};
use barnacle_lib::{Repository, repository::Game};
use iced::{
    Element, Length, Task,
    widget::{Column, button, column, container, row, scrollable, space, text},
};
use tokio::task::spawn_blocking;

mod new_game_dialog;
mod profiles_tab;

const TAB_PADDING: u16 = 16;

#[derive(Debug, Clone)]
pub enum Message {
    StateChanged(State),
    TabSelected(TabId),
    NewGameButtonPressed,
    CloseButtonSelected,
    // Components
    NewGameDialog(new_game_dialog::Message), // ProfilesTab(profiles_tab::Message),
}

/// Action used for communicating with the parent component
#[derive(Debug)]
pub enum Action {
    None,
    Run(Task<Message>),
    AddGame(NewGame),
    DeleteGame(Game),
    Close,
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub enum TabId {
    #[default]
    Profiles,
}

#[derive(Debug, Clone)]
pub enum State {
    Loading,
    Error(String),
    NoGames,
    Loaded {
        active_game: Game,
        games: Vec<GameRow>,
    },
}

pub struct LibraryManager {
    repo: Repository,
    state: State,
    active_tab: TabId,
    // Components
    new_game_dialog: NewGameDialog,
    // profiles_tab: profiles_tab::Tab,
}

impl LibraryManager {
    pub fn new(repo: Repository) -> (Self, Task<Message>) {
        // let (profiles_tab, profiles_task) = profiles_tab::Tab::new(repo.clone());
        let (new_game_dialog, new_game_dialog_task) = new_game_dialog::Dialog::new(repo.clone());

        (
            Self {
                repo: repo.clone(),
                state: State::Loading,
                active_tab: TabId::default(),
                new_game_dialog: NewGameDialog {
                    dialog: new_game_dialog,
                    visible: false,
                },
                // profiles_tab,
            },
            Task::batch([
                new_game_dialog_task.map(Message::NewGameDialog),
                load_state(repo),
            ]),
        )
    }

    pub fn refresh(&self) -> Task<Message> {
        load_state(self.repo.clone())
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::StateChanged(state) => {
                self.state = state;
                Action::None
            }
            Message::TabSelected(id) => {
                self.active_tab = id;
                Action::None
            }
            Message::CloseButtonSelected => Action::Close,
            Message::NewGameButtonPressed => {
                self.new_game_dialog.visible = true;
                Action::None
            }
            Message::NewGameDialog(message) => match self.new_game_dialog.dialog.update(message) {
                new_game_dialog::Action::None => Action::None,
                new_game_dialog::Action::Run(task) => Action::Run(task.map(Message::NewGameDialog)),
                new_game_dialog::Action::AddGame(new_game) => {
                    self.new_game_dialog.visible = false;
                    Action::AddGame(new_game)
                }
                new_game_dialog::Action::Cancel => {
                    self.new_game_dialog.visible = false;
                    Action::None
                }
            },
            // Message::ProfilesTab(message) => match self.profiles_tab.update(message) {
            //     profiles_tab::Action::None => Action::None,
            //     profiles_tab::Action::Run(task) => Action::Run(task.map(Message::ProfilesTab)),
            // },
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let games_sidebar: Element<'_, Message> = match &self.state {
            State::Loading => text("Loading...").into(),
            State::Error(e) => text(e).into(),
            State::NoGames => column![
                text("No games"),
                button("New").on_press(Message::NewGameButtonPressed)
            ]
            .into(),
            State::Loaded { active_game, games } => column![
                scrollable(Column::with_children(games.iter().map(|row| {
                    button(text(row.name.clone())).width(Length::Fill).into()
                }),)),
                button("New").on_press(Message::NewGameButtonPressed)
            ]
            .into(),
        };

        let content = container(column![
            container(row![
                text("Library Manager"),
                space::horizontal(),
                button(icon("close")).on_press(Message::CloseButtonSelected)
            ]),
            row![
                column![text("Games"), games_sidebar].width(Length::FillPortion(1)),
                column![
                    row![
                        button("Profiles").on_press(Message::TabSelected(TabId::Profiles)),
                        space::horizontal(),
                    ],
                    text("Tabs")
                ]
                .width(Length::FillPortion(2))
            ]
        ])
        .width(1000)
        .height(800)
        .style(container::rounded_box)
        .into();

        if self.new_game_dialog.visible {
            modal(
                content,
                self.new_game_dialog
                    .dialog
                    .view()
                    .map(Message::NewGameDialog),
                None,
            )
        } else {
            content
        }
    }
}

fn load_state(repo: Repository) -> Task<Message> {
    let repo = repo.clone();
    Task::perform(
        async {
            spawn_blocking(move || {
                let active_game = repo.active_game().unwrap();
                let games: Vec<GameRow> = repo
                    .games()
                    .unwrap()
                    .iter()
                    .map(|g| GameRow {
                        entity: g.clone(),
                        name: g.name().unwrap(),
                    })
                    .collect();

                if !games.is_empty() {
                    State::Loaded {
                        active_game: active_game.unwrap(),
                        games,
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

#[derive(Debug, Clone)]
struct GameRow {
    entity: Game,
    name: String,
}

#[derive(Debug, Clone)]
struct NewGameDialog {
    dialog: new_game_dialog::Dialog,
    visible: bool,
}
