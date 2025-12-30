use crate::{components::library_manager::new_game_dialog::NewGame, icons::icon, modal};
use adisruption_widgets::generic_overlay::{self, overlay_button};
use barnacle_lib::{Repository, repository::Game};
use derive_more::Deref;
use iced::{
    Element, Length, Task,
    widget::{Column, button, column, container, row, rule, scrollable, space, text},
};
use tokio::task::spawn_blocking;

mod new_game_dialog;
mod profiles_tab;

#[derive(Debug, Clone)]
pub enum Message {
    StateChanged(State),
    TabSelected(TabId),
    AddGameButtonPressed,
    CloseButtonPressed,
    GameRowSelected(Game),
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
    Overview,
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
    selected_game: Option<Game>,
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
                selected_game: None,
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
            Message::CloseButtonPressed => Action::Close,
            Message::AddGameButtonPressed => {
                self.new_game_dialog.visible = true;
                Action::None
            }
            Message::GameRowSelected(game) => {
                self.selected_game = Some(game);
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
        let add_game_button = overlay_button(
            row![icon("plus"), text(" Add Game")],
            "Add Game",
            self.new_game_dialog
                .dialog
                .view()
                .map(Message::NewGameDialog),
        );

        match &self.state {
            State::Loading => text("Loading...").into(),
            State::Error(e) => text(e).into(),
            State::NoGames => column![text("No games"), add_game_button].into(),
            State::Loaded { active_game, games } => {
                let game_rows = games
                    .iter()
                    .map(|row| game_row(row, active_game, &self.selected_game));

                let games_sidebar = column![
                    scrollable(Column::with_children(game_rows)),
                    space::vertical(),
                    add_game_button
                ];

                row![
                    column![text("Games"), rule::horizontal(1), games_sidebar]
                        .width(Length::FillPortion(1)),
                    column![
                        row![
                            button("Profiles").on_press(Message::TabSelected(TabId::Profiles)),
                            space::horizontal(),
                        ],
                        text("Tabs")
                    ]
                    .width(Length::FillPortion(2))
                ]
                .into()
            }
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

// Generate a row that represents a Game
fn game_row<'a>(
    row: &'a GameRow,
    active_game: &'a Game,
    selected_game: &'a Option<Game>,
) -> Element<'a, Message> {
    let mut content = row![text(row.name.clone()), space::horizontal()];

    if &row.entity == active_game {
        content = content.push(icon("check"));
    }

    let style = if Some(&row.entity) == selected_game.as_ref() {
        button::primary
    } else {
        button::subtle
    };

    button(content)
        .width(Length::Fill)
        .style(style)
        .on_press(Message::GameRowSelected(row.entity.clone()))
        .into()
}

#[derive(Debug, Clone)]
pub struct GameRow {
    entity: Game,
    name: String,
}

#[derive(Debug, Clone, Deref)]
struct NewGameDialog {
    #[deref]
    dialog: new_game_dialog::Dialog,
    visible: bool,
}
