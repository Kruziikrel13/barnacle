use barnacle_gui::icons::icon;
use barnacle_lib::{
    Repository,
    repository::{Profile, entities::ModEntry},
};
use iced::{
    Element, Length, Task,
    widget::{Svg, button, checkbox, column, row, scrollable, space, table, text},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Vec<ModEntry>),
    SortChanged(SortColumn),
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
    sort: SortState,
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
                sort: SortState::default(),
            },
            task,
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loaded(entries) => self.state = State::Loaded(entries),
            Message::SortChanged(column) => self.sort = self.sort.toggle(column),
            Message::ModEntryToggled(mut entry, state) => {
                // TODO: This should be async
                entry.set_enabled(state).unwrap();
            }
            Message::ModEntryDeleted(entry) => {
                let current_profile = self.repo.clone().current_profile().unwrap();
                current_profile.remove_mod_entry(entry).unwrap();
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
                        column_header("Name", &self.sort, SortColumn::Name),
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

fn column_header<'a>(
    name: &'a str,
    sort_state: &'a SortState,
    column: SortColumn,
) -> Element<'a, Message> {
    button(row![
        text(name),
        space::horizontal(),
        sort_state.icon(column)
    ])
    .style(button::subtle)
    .width(Length::Fill)
    .on_press(Message::SortChanged(column))
    .into()
}

fn update_mods_list(profile: &Profile) -> Task<Message> {
    Task::perform(
        {
            let profile = profile.clone();
            async move { profile.mod_entries().unwrap() }
        },
        Message::Loaded,
    )
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub enum SortColumn {
    Name,
    // Add more columns here later
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortState {
    column: SortColumn,
    direction: SortDirection,
}

impl SortState {
    fn toggle(&self, column: SortColumn) -> Self {
        if self.column == column {
            let new_direction = match self.direction {
                SortDirection::Ascending => SortDirection::Descending,
                SortDirection::Descending => SortDirection::Ascending,
            };

            Self {
                column,
                direction: new_direction,
            }
        } else {
            // A different column than the currently sorted one has been selected
            Self {
                column,
                ..Default::default()
            }
        }
    }

    fn icon(&'_ self, column: SortColumn) -> Option<Svg<'_>> {
        if self.column == column {
            Some(match self.direction {
                SortDirection::Ascending => icon("arrow_up"),
                SortDirection::Descending => icon("arrow_down"),
            })
        } else {
            None
        }
    }
}

impl Default for SortState {
    fn default() -> Self {
        Self {
            column: SortColumn::Name,
            direction: SortDirection::Ascending,
        }
    }
}
