use crate::icons::icon;
use barnacle_lib::Repository;
use iced::{
    Element, Task,
    widget::{button, column, container, row, space},
};
use tokio::task::spawn_blocking;

mod games_tab;
mod profiles_tab;

const TAB_PADDING: u16 = 16;

#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(TabId),
    CloseButtonSelected,
    GameAdded,
    GameDeleted,
    // Components
    GamesTab(games_tab::Message),
    ProfilesTab(profiles_tab::Message),
    GameEdited,
}

/// Action used for communicating with the parent component
#[derive(Debug)]
pub enum Action {
    None,
    Run(Task<Message>),
    Close,
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub enum TabId {
    #[default]
    Games,
    Profiles,
}

pub struct LibraryManager {
    repo: Repository,
    active_tab: TabId,
    // Components
    games_tab: games_tab::Tab,
    profiles_tab: profiles_tab::Tab,
}

impl LibraryManager {
    pub fn new(repo: Repository) -> (Self, Task<Message>) {
        let (games_tab, games_task) = games_tab::Tab::new(repo.clone());
        let (profiles_tab, profiles_task) = profiles_tab::Tab::new(repo.clone());

        let tasks = Task::batch([
            games_task.map(Message::GamesTab),
            profiles_task.map(Message::ProfilesTab),
        ]);

        (
            Self {
                repo: repo.clone(),
                active_tab: TabId::default(),
                games_tab,
                profiles_tab,
            },
            tasks,
        )
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::TabSelected(id) => {
                self.active_tab = id;
                Action::None
            }
            Message::CloseButtonSelected => Action::Close,
            // TODO: Profiles tab game selection combo box doesn't get updated about newly created games
            Message::GamesTab(message) => match self.games_tab.update(message) {
                games_tab::Action::None => Action::None,
                games_tab::Action::Run(task) => Action::Run(task.map(Message::GamesTab)),
                games_tab::Action::AddGame { name, deploy_kind } => Action::Run(Task::perform(
                    {
                        let repo = self.repo.clone();
                        async move { spawn_blocking(move || repo.add_game(&name, deploy_kind)).await }
                    },
                    |_| Message::GameAdded,
                )),
                games_tab::Action::EditGame {
                    game,
                    name,
                    deploy_kind,
                } => {
                    let deploy_kind = deploy_kind.clone();
                    Action::Run(Task::perform(
                        async move {
                            spawn_blocking(move || {
                                game.set_name(&name).unwrap();
                                game.set_deploy_kind(deploy_kind).unwrap();
                            })
                            .await
                            .unwrap()
                        },
                        |_| Message::GameEdited,
                    ))
                }
                games_tab::Action::DeleteGame(game) => Action::Run(Task::perform(
                    {
                        let repo = self.repo.clone();
                        async move { spawn_blocking(move || repo.remove_game(game).unwrap()).await }
                    },
                    |_| Message::GameDeleted,
                )),
            },
            Message::ProfilesTab(message) => match self.profiles_tab.update(message) {
                profiles_tab::Action::None => Action::None,
                profiles_tab::Action::Run(task) => Action::Run(task.map(Message::ProfilesTab)),
            },
            Message::GameAdded | Message::GameEdited | Message::GameDeleted => {
                Action::Run(Task::batch([
                    self.games_tab.refresh().map(Message::GamesTab),
                    self.profiles_tab.refresh().map(Message::ProfilesTab),
                ]))
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        container(column![
            row![
                button("Games").on_press(Message::TabSelected(TabId::Games)),
                button("Profiles").on_press(Message::TabSelected(TabId::Profiles)),
                space::horizontal(),
                button(icon("close")).on_press(Message::CloseButtonSelected)
            ],
            match self.active_tab {
                TabId::Games => self.games_tab.view().map(Message::GamesTab),
                TabId::Profiles => self.profiles_tab.view().map(Message::ProfilesTab),
            },
        ])
        .width(1000)
        .height(800)
        .style(container::rounded_box)
        .into()
    }
}
