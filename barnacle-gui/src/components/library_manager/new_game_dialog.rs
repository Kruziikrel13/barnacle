use barnacle_lib::{Repository, repository::DeployKind};
use iced::{
    Element, Task,
    widget::{button, column, combo_box, container, row, space, text, text_input},
};
use strum::IntoEnumIterator;

#[derive(Debug, Clone)]
pub enum Message {
    NameInput(String),
    DeployKindSelected(DeployKind),
    CancelPressed,
    CreatePressed,
    GameCreated,
}

#[derive(Debug)]
pub struct NewGame {
    pub name: String,
    pub deploy_kind: DeployKind,
}

#[derive(Debug)]
pub enum Action {
    None,
    Run(Task<Message>),
    Cancel,
    AddGame(NewGame),
}

#[derive(Debug, Clone)]
pub struct Dialog {
    repo: Repository,
    name: String,
    deploy_kind: Option<DeployKind>,
    deploy_kind_state: combo_box::State<DeployKind>,
}

impl Dialog {
    pub fn new(repo: Repository) -> (Self, Task<Message>) {
        (
            Self {
                repo,
                name: "".into(),
                deploy_kind: None,
                deploy_kind_state: combo_box::State::new(DeployKind::iter().collect()),
            },
            Task::none(),
        )
    }

    /// Reset the dialog state
    pub fn clear(&mut self) {
        self.name.clear();
        self.deploy_kind = None;
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::NameInput(content) => {
                self.name = content;
                Action::None
            }
            Message::DeployKindSelected(kind) => {
                self.deploy_kind = Some(kind);
                Action::None
            }
            Message::CancelPressed => {
                self.clear();
                Action::Cancel
            }
            Message::CreatePressed => {
                let name = self.name.clone();
                let deploy_kind = self.deploy_kind.unwrap();

                self.clear();

                Action::AddGame(NewGame {
                    name,
                    // TODO: Make deploy kind required instead of crashing w/o it
                    deploy_kind,
                })
            }
            Message::GameCreated => Action::None,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        container(column![
            row![
                text("Name: "),
                text_input("Name", &self.name).on_input(Message::NameInput),
            ],
            row![
                text("Deploy kind: "),
                combo_box(
                    &self.deploy_kind_state,
                    "Select a deploy kind",
                    self.deploy_kind.as_ref(),
                    Message::DeployKindSelected
                ),
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
