use adisruption_widgets::generic_overlay;
use iced::{
    Element, Task,
    advanced::widget::operate,
    widget::{button, column, container, row, space, text, text_input},
};

pub const ID: &str = "new_profile_dialog";

#[derive(Debug, Clone)]
pub enum Message {
    NameInput(String),
    CancelPressed,
    CreatePressed,
}

pub enum Action {
    None,
    Run(Task<Message>),
    Create(NewProfile),
}

pub struct NewDialog {
    name: String,
}

#[derive(Debug, Clone)]
pub struct NewProfile {
    pub name: String,
}

impl NewDialog {
    pub fn new() -> (Self, Task<Message>) {
        (Self { name: "".into() }, Task::none())
    }

    /// Reset the dialog state
    pub fn clear(&mut self) {
        self.name.clear();
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::NameInput(content) => {
                self.name = content;
                Action::None
            }
            Message::CancelPressed => {
                self.clear();
                Action::Run(operate(generic_overlay::close::<Message>(ID.into())))
            }
            Message::CreatePressed => {
                let name = self.name.clone();

                self.clear();

                Action::Create(NewProfile { name })
            }
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
        .into()
    }
}
