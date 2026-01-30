use barnacle_lib::Repository;
use clap::Subcommand;

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// List profiles
    List,
    /// Add a new profile
    Add { name: String },
}

pub fn handle(repo: &Repository, cmd: &Command) {
    let active_game = repo.active_game().unwrap().unwrap();
    match cmd {
        Command::List => {
            let profiles = active_game.profiles().unwrap();
            for profile in profiles {
                println!("{}", profile.name().unwrap());
            }
        }
        Command::Add { name } => {
            active_game.add_profile(name).unwrap();
        }
    }
}
