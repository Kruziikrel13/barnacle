use crate::{
    components::library_manager::games_sidebar::{edit_dialog::GameEdit, new_dialog::NewGame},
    icons::icon,
};
use barnacle_lib::{Repository, repository::Game};
use iced::{
    Element, Length, Task,
    widget::{button, column, container, row, space},
};

mod games_sidebar;
mod profiles_tab;

const TAB_PADDING: u16 = 16;

#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(TabId),
    CloseButtonSelected,
    // Components
    GamesSidebar(games_sidebar::Message),
    ProfilesTab(profiles_tab::Message),
}

/// Action used for communicating with the parent component
#[derive(Debug)]
pub enum Action {
    None,
    Run(Task<Message>),
    AddGame(NewGame),
    EditGame(GameEdit),
    DeleteGame(Game),
    Close,
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub enum TabId {
    #[default]
    Profiles,
}

pub struct LibraryManager {
    repo: Repository,
    active_tab: TabId,
    // Components
    games_sidebar: games_sidebar::Tab,
    profiles_tab: profiles_tab::Tab,
}

impl LibraryManager {
    pub fn new(repo: Repository) -> (Self, Task<Message>) {
        let (games_sidebar, games_task) = games_sidebar::Tab::new(repo.clone());
        let (profiles_tab, profiles_task) = profiles_tab::Tab::new(repo.clone());

        let tasks = Task::batch([
            games_task.map(Message::GamesSidebar),
            profiles_task.map(Message::ProfilesTab),
        ]);

        (
            Self {
                repo: repo.clone(),
                active_tab: TabId::default(),
                games_sidebar,
                profiles_tab,
            },
            tasks,
        )
    }

    pub fn refresh(&self) -> Task<Message> {
        Task::batch([
            self.games_sidebar.refresh().map(Message::GamesSidebar),
            self.profiles_tab.refresh().map(Message::ProfilesTab),
        ])
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::TabSelected(id) => {
                self.active_tab = id;
                Action::None
            }
            Message::CloseButtonSelected => Action::Close,
            // TODO: Profiles tab game selection combo box doesn't get updated about newly created games
            Message::GamesSidebar(message) => match self.games_sidebar.update(message) {
                games_sidebar::Action::None => Action::None,
                games_sidebar::Action::Run(task) => Action::Run(task.map(Message::GamesSidebar)),
                games_sidebar::Action::AddGame(new_game) => Action::AddGame(new_game),
                games_sidebar::Action::EditGame(game_edit) => Action::EditGame(game_edit),
                games_sidebar::Action::DeleteGame(game) => Action::DeleteGame(game),
            },
            Message::ProfilesTab(message) => match self.profiles_tab.update(message) {
                profiles_tab::Action::None => Action::None,
                profiles_tab::Action::Run(task) => Action::Run(task.map(Message::ProfilesTab)),
            },
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        container(row![
            column![self.games_sidebar.view().map(Message::GamesSidebar)]
                .width(Length::FillPortion(1)),
            column![
                row![
                    button("Profiles").on_press(Message::TabSelected(TabId::Profiles)),
                    space::horizontal(),
                    button(icon("close")).on_press(Message::CloseButtonSelected)
                ],
                match self.active_tab {
                    TabId::Profiles => self.profiles_tab.view().map(Message::ProfilesTab),
                },
            ]
            .width(Length::FillPortion(2))
        ])
        .width(1000)
        .height(800)
        .style(container::rounded_box)
        .into()
    }
}
