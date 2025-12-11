use barnacle_lib::{Repository, repository::entities::ModEntry};
use iced::{
    Element, Length, Task,
    widget::{checkbox, column, scrollable, table, text},
};

#[derive(Debug, Clone)]
pub struct ModEntryRow {
    name: String,
    notes: String,
    enabled: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Vec<ModEntry>),
}

pub enum State {
    Loading,
    Error(String),
    Loaded(Vec<ModEntryRow>),
}

pub struct ModList {
    repo: Repository,
    state: State,
}

impl ModList {
    pub fn new(repo: Repository) -> (Self, Task<Message>) {
        let task = Task::perform(
            {
                let repo = repo.clone();
                async move {
                    let current_profile = repo.clone().current_profile().unwrap();
                    current_profile.mod_entries().unwrap()
                }
            },
            Message::Loaded,
        );

        (
            Self {
                repo,
                state: State::Loading,
            },
            task,
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loaded(mod_entries) => {
                let rows = mod_entries
                    .iter()
                    .map(|m| ModEntryRow {
                        name: m.name().unwrap().to_string(),
                        notes: m.notes().unwrap().to_string(),
                        enabled: m.enabled().unwrap(),
                    })
                    .collect();

                self.state = State::Loaded(rows)
            }
        }

        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        match &self.state {
            State::Loading => column![text("Loading mods...")],
            State::Error(e) => column![text(e)],
            State::Loaded(rows) => {
                let columns = [
                    table::column(text("Name"), |row: ModEntryRow| text(row.name)),
                    table::column(text("Notes"), |row: ModEntryRow| text(row.notes)),
                    table::column(text("Status"), |row: ModEntryRow| checkbox(row.enabled)),
                ];

                column![scrollable(table(columns, rows.clone()).width(Length::Fill))]
            }
        }
        .into()
    }
}
