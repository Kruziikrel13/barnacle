use crate::{config::Cfg, icons::icon};
use barnacle_lib::{
    Repository,
    repository::{Profile, entities::ModEntry},
};
use iced::{
    Element, Length, Point, Task,
    widget::{Svg, button, checkbox, column, mouse_area, row, scrollable, space, table, text},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Vec<ModEntry>),
    SortChanged(SortColumn),
    ModEntryToggled(ModEntry, bool),
    ModEntryHovered(ModEntry, Point),
    ModEntryRightClicked,
    ModEntryDeleted(ModEntry),
}

pub enum State {
    Loading,
    Error(String),
    Loaded(Vec<ModEntry>),
}

pub struct ModList {
    repo: Repository,
    cfg: Cfg,
    state: State,
    sort: SortState,
    context_menu: ContextMenuState,
}

impl ModList {
    pub fn new(repo: Repository, cfg: Cfg) -> (Self, Task<Message>) {
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
                cfg,
                state: State::Loading,
                sort: SortState::default(),
                context_menu: ContextMenuState {
                    visible: false,
                    entry: None,
                    position: None,
                },
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
            Message::ModEntryHovered(entry, position) => {
                self.context_menu.entry = Some(entry);
                self.context_menu.position = Some(position);
            }
            Message::ModEntryRightClicked => {
                self.context_menu.visible = true;
            }
            Message::ModEntryDeleted(entry) => {
                println!("Deletion of {:?}", entry);
                // let current_profile = self.repo.clone().current_profile().unwrap();
                // current_profile.remove_mod_entry(entry).unwrap();
            }
        }

        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        match &self.state {
            State::Loading => column![text("Loading mods...")].into(),
            State::Error(e) => column![text(e)].into(),
            State::Loaded(mod_entries) => {
                let columns = [
                    table::column(
                        column_header("Name", &self.sort, SortColumn::Name),
                        |entry: ModEntry| {
                            mouse_area(text(entry.name().unwrap()))
                                .on_right_press(Message::ModEntryRightClicked)
                                .on_move(move |p| Message::ModEntryHovered(entry.clone(), p))
                        },
                    ),
                    table::column(text("Status"), |entry: ModEntry| {
                        checkbox(entry.enabled().unwrap())
                            .on_toggle(move |state| Message::ModEntryToggled(entry.clone(), state))
                    }),
                ];

                let content = column![scrollable(
                    table(columns, mod_entries.clone()).width(Length::Fill)
                )];

                content.into()

                // if self.context_menu.visible {
                //     ContextMenu::new(content, || {
                //         column![button("Delete").on_press(Message::ModEntryDeleted(
                //             self.context_menu.entry.clone().unwrap()
                //         ))]
                //         .into()
                //     })
                //     .into()
                // } else {
                //     content.into()
                // }
            }
        }
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

#[derive(Debug, Clone)]
pub struct ContextMenuState {
    visible: bool,
    entry: Option<ModEntry>,
    position: Option<Point>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub enum SortColumn {
    Name,
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
