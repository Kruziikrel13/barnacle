use barnacle_lib::{Repository, repository::entities::ModEntry};
use iced::{
    Element, Length, Task,
    widget::{checkbox, column, scrollable, table, text},
};

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Vec<ModEntry>),
    ModEntryToggled(ModEntry, bool),
}

pub enum State {
    Loading,
    Error(String),
    Loaded(Vec<ModEntry>),
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
            Message::Loaded(entries) => self.state = State::Loaded(entries),
            Message::ModEntryToggled(mut entry, state) => {
                entry.set_enabled(state).unwrap();
            }
        }

        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        match &self.state {
            State::Loading => column![text("Loading mods...")],
            State::Error(e) => column![text(e)],
            State::Loaded(mod_entries) => {
                let columns = [
                    table::column(text("Name"), |entry: ModEntry| text(entry.name().unwrap())),
                    table::column(text("Notes"), |entry: ModEntry| {
                        text(entry.notes().unwrap())
                    }),
                    table::column(text("Status"), |entry: ModEntry| {
                        checkbox(entry.enabled().unwrap())
                            .on_toggle(move |state| Message::ModEntryToggled(entry.clone(), state))
                    }),
                ];

                column![scrollable(
                    table(columns, mod_entries.clone()).width(Length::Fill)
                )]
            }
        }
        .into()
    }
}
