use barnacle_lib::repository::Game;
use iced::{
    Element, Task,
    widget::{button, column, container, row, space, text, text_input},
};

#[derive(Debug, Clone)]
pub enum Message {
    NameInput(String),
    CancelPressed,
    CreatePressed,
    ProfileCreated,
}

pub struct NewDialog {
    game: Option<Game>,
    name: String,
}

impl NewDialog {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                game: None,
                name: "".into(),
            },
            Task::none(),
        )
    }

    pub fn load(&mut self, game: Game) {
        self.game = Some(game);
        self.name = "".into();
    }

    /// Reset the dialog state
    pub fn clear(&mut self) {
        self.name.clear();
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        if let Some(game) = &self.game {
            match message {
                Message::NameInput(content) => {
                    self.name = content;
                    Task::none()
                }
                Message::CancelPressed => Task::none(),
                Message::CreatePressed => {
                    let mut game = game.clone();
                    let name = self.name.clone();

                    self.clear();

                    Task::perform(
                        async move {
                            game.add_profile(&name).unwrap();
                        },
                        |_| Message::ProfileCreated,
                    )
                }
                Message::ProfileCreated => Task::none(),
            }
        } else {
            Task::none()
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        container(column![
            row![
                text("Name: "),
                text_input("Name", &self.name).on_input(Message::NameInput),
            ],
            space::vertical(),
            row![
                space::horizontal(),
                button("Cancel").on_press(Message::CancelPressed),
                button("Create").on_press(Message::CreatePressed),
            ],
        ])
        .padding(20)
        .width(400)
        .height(600)
        .style(container::rounded_box)
        .into()
    }
}
