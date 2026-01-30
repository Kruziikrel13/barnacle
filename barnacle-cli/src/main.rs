use barnacle_lib::Repository;
use clap::{Parser, Subcommand};

mod game;
mod profile;

#[derive(Parser, Debug)]
#[command(name = "barnacle")]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Override the active game
    #[arg(short, long, global = true)]
    game: Option<String>,

    /// Override the active profile
    #[arg(short, long, global = true)]
    profile: Option<String>,
}

#[derive(Subcommand, Debug, Clone)]
enum Command {
    /// Operate on games
    #[command(subcommand)]
    Game(game::Command),
    /// Operate on profiles
    #[command(subcommand)]
    Profile(profile::Command),
}

fn main() {
    let repo = Repository::new();
    let cli = Cli::parse();

    match &cli.command {
        Command::Game(cmd) => game::handle(&repo, cmd),
        Command::Profile(cmd) => profile::handle(&repo, cmd),
    }
}
