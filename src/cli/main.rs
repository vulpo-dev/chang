use clap::{Parser, Subcommand};
use dotenv::dotenv;

mod migrate;

use migrate::MigrateArgs;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Migrate(MigrateArgs),
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let cli = Cli::parse();

    if let Some(command) = cli.command {
        match command {
            Commands::Migrate(args) => migrate::run(args).await,
        }
    }
}
