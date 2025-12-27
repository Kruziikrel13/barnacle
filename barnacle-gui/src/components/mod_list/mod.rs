use crate::{
    components::mod_list::state::{ContextMenuState, SortColumn, SortState},
    config::Cfg,
};
use barnacle_lib::{Repository, repository::entities::ModEntry};
use iced::{
    Element, Length, Point, Task,
    widget::{button, checkbox, column, row, scrollable, table, text},
};
use sweeten::widget::mouse_area;
use tokio::task::spawn_blocking;

pub mod state;

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Vec<ModEntry>),
    SortChanged(SortColumn),
    ClickedOutContextMenu,
    ModEntryToggled(ModEntry, bool),
    ModEntryRightClicked(ModEntry, Point),
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
    context_menu: Option<ContextMenuState>,
}

impl ModList {
    pub fn new(repo: Repository, cfg: Cfg) -> (Self, Task<Message>) {
        (
            Self {
                repo: repo.clone(),
                cfg,
                state: State::Loading,
                sort: SortState::default(),
                context_menu: None,
            },
            list_mods(&repo),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loaded(entries) => self.state = State::Loaded(entries),
            Message::SortChanged(column) => {
                self.sort = self.sort.toggle(column);
                self.cfg.write().mod_list.sort_state = self.sort;
            }
            Message::ClickedOutContextMenu => self.context_menu = None,
            Message::ModEntryToggled(entry, state) => {
                // TODO: This should be async
                entry.set_enabled(state).unwrap();
            }
            Message::ModEntryRightClicked(entry, position) => {
                self.context_menu = Some(ContextMenuState::new(entry, position))
            }
            Message::ModEntryDeleted(entry) => {
                println!("Deletion of {:?}", entry);
                // entry.remove().unwrap();
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
                            mouse_area(text(entry.name().unwrap())).on_right_press(move |point| {
                                Message::ModEntryRightClicked(entry.clone(), point)
                            })
                        },
                    ),
                    table::column(
                        column_header("Cateogry", &self.sort, SortColumn::Category),
                        |entry: ModEntry| text("Category"),
                    ),
                    table::column(text("Status"), |entry: ModEntry| {
                        checkbox(entry.enabled().unwrap())
                            .on_toggle(move |state| Message::ModEntryToggled(entry.clone(), state))
                    }),
                ];

                column![scrollable(
                    table(columns, mod_entries.clone()).width(Length::Fill)
                )]
                .into()
            }
        }
    }

    pub fn refresh(&self) -> Task<Message> {
        list_mods(&self.repo)
    }
}

fn list_mods(repo: &Repository) -> Task<Message> {
    let repo = repo.clone();
    Task::perform(
        async {
            spawn_blocking(move || {
                if let Some(profile) = repo.clone().active_profile().unwrap() {
                    profile.mod_entries().unwrap()
                } else {
                    Vec::new()
                }
            })
            .await
            .unwrap()
        },
        Message::Loaded,
    )
}

fn column_header<'a>(
    name: &'a str,
    sort_state: &'a SortState,
    column: SortColumn,
) -> Element<'a, Message> {
    button(row![text(name), sort_state.icon(column)])
        .style(button::subtle)
        .on_press(Message::SortChanged(column))
        .into()
}
