use barnacle_lib::{Repository, repository::DeployKind};
use clap::Subcommand;

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// List games
    List,
    /// Add a new game
    Add { name: String },
    /// Activate the given game
    Activate { name: String },
}

pub fn handle(repo: &Repository, cmd: &Command) {
    match cmd {
        Command::List => {
            let games = repo.games().unwrap();
            for game in games {
                println!("{}", game.name().unwrap());
            }
        }
        Command::Add { name } => {
            repo.add_game(name, DeployKind::Overlay).unwrap();
        }
        Command::Activate { name } => {
            let game = repo.search_game(name).unwrap().expect("game not found");
            game.activate().unwrap();
        }
    }
}
