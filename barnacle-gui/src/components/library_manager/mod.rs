use crate::icons::icon;
use barnacle_lib::Repository;
use iced::{
    Element, Task,
    widget::{button, column, container, row, space},
};

mod games_tab;
mod profiles_tab;

const TAB_PADDING: u16 = 16;

#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(TabId),
    CloseButtonSelected,
    // Components
    GamesTab(games_tab::Message),
    ProfilesTab(profiles_tab::Message),
}

/// Event used for communicating with the parent component
#[derive(Debug)]
pub enum Event {
    None,
    Task(Task<Message>),
    Closed,
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

    pub fn update(&mut self, message: Message) -> Event {
        match message {
            Message::TabSelected(id) => {
                self.active_tab = id;
                Event::None
            }
            Message::CloseButtonSelected => Event::Closed,
            // TODO: Profiles tab game selection combo box doesn't get updated about newly created games
            Message::GamesTab(message) => match self.games_tab.update(message) {
                games_tab::Event::None => Event::None,
                games_tab::Event::Task(task) => Event::Task(task.map(Message::GamesTab)),
                games_tab::Event::GameDeleted => {
                    println!("Game deleted");
                    Event::None
                }
            },
            Message::ProfilesTab(msg) => {
                Event::Task(self.profiles_tab.update(msg).map(Message::ProfilesTab))
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
