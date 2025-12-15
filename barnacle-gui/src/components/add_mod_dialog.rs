use barnacle_lib::Repository;
use iced::{
    Element, Task,
    widget::{button, column, container, row, space, text_input},
};
use rfd::AsyncFileDialog;

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    PathChanged(String),
    PickPathButtonPressed,
    PathPicked(Option<String>),
    CancelButtonPressed,
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

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::NameChanged(name) => {
                self.name = name;
                Task::none()
            }
            Message::PathChanged(path) => {
                self.path = path;
                Task::none()
            }
            Message::PickPathButtonPressed => Task::perform(
                async {
                    AsyncFileDialog::new()
                        .set_directory("/")
                        .pick_file()
                        .await
                        .map(|handle| handle.path().display().to_string())
                },
                Message::PathPicked,
            ),
            Message::PathPicked(path) => {
                if let Some(path) = path {
                    self.path = path;
                }
                Task::none()
            }
            // Handled higher up
            Message::CancelButtonPressed => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        container(column![
            row![
                "Name: ",
                text_input("Name...", &self.name).on_input(Message::NameChanged)
            ],
            row![
                "Path: ",
                text_input("Path...", &self.path).on_input(Message::PathChanged),
                button("...").on_press(Message::PickPathButtonPressed)
            ],
            space::vertical(),
            row![
                space::horizontal(),
                button("Cancel").on_press(Message::CancelButtonPressed),
                button("Add")
            ]
        ])
        .padding(20)
        .width(400)
        .height(600)
        .style(container::rounded_box)
        .into()
    }
}
