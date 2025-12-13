use barnacle_lib::{Repository, repository::entities::ModEntry};
use iced::{
    Element, Length, Renderer, Task, Theme,
    widget::{button, checkbox, column, scrollable, table, text},
};

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Vec<ModEntry>),
    ModEntryToggled(ModEntry, bool),
    ModEntryDeleted(ModEntry),
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
                // TODO: This should be async
                entry.set_enabled(state).unwrap();
            }
            Message::ModEntryDeleted(entry) => {
                let current_profile = self.repo.clone().current_profile().unwrap();
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
                    table::column(
                        // TODO: Make clicking on the header sort this guy
                        button("Name").style(button::subtle).width(Length::Fill),
                        |entry: ModEntry| text(entry.name().unwrap()),
                    ),
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

fn context_menu<'a, U>(underlay: U)
where
    U: Into<Element<'a, Message, Theme, Renderer>>,
{
}
