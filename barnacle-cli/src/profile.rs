use barnacle_lib::Repository;
use clap::Subcommand;
use cliux::List;

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// List profiles
    List,
    /// Add a new profile
    Add { name: String },
}

pub fn handle(repo: &Repository, cmd: &Command) {
    if let Some(active_game) = repo.active_game().unwrap() {
        match cmd {
            Command::List => {
                let profiles: Vec<String> = active_game
                    .profiles()
                    .unwrap()
                    .into_iter()
                    .map(|p| p.name().unwrap())
                    .collect();
            }
            Command::Add { name } => {
                active_game.add_profile(name).unwrap();
            }
        }
    } else {
        println!("No active game")
    }
}
