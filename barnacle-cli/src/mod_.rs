use std::path::Path;

use barnacle_lib::Repository;
use clap::Subcommand;

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// List profiles
    List,
    /// Add a new profile
    Add { name: String, path: Option<String> },
}

pub struct ModRow {
    name: String,
    enabled: bool,
}

pub fn handle(repo: &Repository, cmd: &Command) {
    if let Some(active_game) = repo.active_game().unwrap() {
        if let Some(active_profile) = active_game.active_profile().unwrap() {
            match cmd {
                Command::List => {
                    let mods = active_profile.mod_entries().unwrap();
                    for mod_ in mods {
                        println!("* {}", mod_.name().unwrap());
                    }
                }
                Command::Add { name, path } => {
                    let mod_ = active_game
                        .add_mod(name, path.as_deref().map(Path::new))
                        .unwrap();
                    active_profile.add_mod_entry(mod_).unwrap();
                }
            }
        } else {
            eprintln!("No active profile");
        }
    } else {
        eprintln!("No active game")
    }
}
