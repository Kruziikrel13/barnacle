use barnacle_lib::Repository;
use clap::Subcommand;

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// List profiles
    List,
    /// Add a new profile
    Add { name: String },
    /// Activate the given profile
    Activate { name: String },
}

pub fn handle(repo: &Repository, cmd: &Command) {
    if let Some(active_game) = repo.active_game().unwrap() {
        match cmd {
            Command::List => {
                let profiles = active_game.profiles().unwrap();
                for profile in profiles {
                    println!("* {}", profile.name().unwrap())
                }
            }
            Command::Add { name } => {
                active_game.add_profile(name).unwrap();
            }
            Command::Activate { name } => {
                let profile = active_game
                    .search_profile(name)
                    .unwrap()
                    .expect("profile not found");
                profile.activate().unwrap();
            }
        }
    } else {
        println!("No active game")
    }
}
