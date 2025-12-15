use std::env;

use barnacle_lib::Repository;
use iced::{
    Element, Task,
    widget::{button, column, container, row, space, text_input},
};
use rfd::AsyncFileDialog;

use crate::icons::icon;

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    PathChanged(String),
    PickPath(PickPathKind),
    PathPicked(Option<String>),
    CancelButtonPressed,
    AddButtonPressed,
}

/// Actions handed to parent widget to be performed
#[derive(Debug)]
pub enum Action {
    None,
    Task(Task<Message>),
    Cancel,
    AddMod,
}

#[derive(Debug, Clone)]
pub enum PickPathKind {
    Archive,
    Directory,
}

pub struct AddModDialog {
    repo: Repository,
    name: String,
    path: String,
}

impl AddModDialog {
    pub fn new(repo: Repository) -> (Self, Task<Message>) {
        (
            Self {
                repo: repo.clone(),
                name: "".into(),
                path: "".into(),
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::NameChanged(name) => {
                self.name = name;
                Action::None
            }
            Message::PathChanged(path) => {
                self.path = path;
                Action::None
            }
            Message::PickPath(kind) => Action::Task(Task::perform(
                async move {
                    let picker = AsyncFileDialog::new().set_directory(env::home_dir().unwrap());

                    match kind {
                        PickPathKind::Archive => picker.pick_file().await,
                        PickPathKind::Directory => picker.pick_folder().await,
                    }
                    .map(|f| f.path().display().to_string())
                },
                Message::PathPicked,
            )),
            Message::PathPicked(path) => {
                if let Some(path) = path {
                    self.path = path;
                }
                Action::None
            }
            Message::CancelButtonPressed => Action::Cancel,
            Message::AddButtonPressed => Action::AddMod,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        container(column![
            row![
                "Name: ",
                text_input("...", &self.name).on_input(Message::NameChanged)
            ],
            row![
                "Path: ",
                text_input("...", &self.path).on_input(Message::PathChanged),
                button(icon("archive")).on_press(Message::PickPath(PickPathKind::Archive)),
                button(icon("directory")).on_press(Message::PickPath(PickPathKind::Directory))
            ],
            space::vertical(),
            row![
                space::horizontal(),
                button("Cancel").on_press(Message::CancelButtonPressed),
                button("Add").on_press(Message::AddButtonPressed)
            ]
        ])
        .padding(20)
        .width(400)
        .height(600)
        .style(container::rounded_box)
        .into()
    }
}
