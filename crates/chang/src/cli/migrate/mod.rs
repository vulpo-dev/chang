use std::env;

use clap::{Args, Subcommand};
use sqlx::postgres::PgPoolOptions;

#[derive(Args)]
pub struct MigrateArgs {
    database_url: Option<String>,

    #[command(subcommand)]
    command: MigrateCommands,
}

#[derive(Subcommand)]
enum MigrateCommands {
    Base,
    Otel,
    Events,
}

pub async fn run(args: MigrateArgs) {
    let database_url = args
        .database_url
        .unwrap_or_else(|| env::var("DATABASE_URL").expect("DATABASE_URL environment variable"));

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("Valid DB connection");

    match args.command {
        MigrateCommands::Base => chang_core::db::migration::base(&pool).await,
        MigrateCommands::Otel => chang_core::db::migration::otel(&pool).await,
        MigrateCommands::Events => chang_core::db::migration::events(&pool).await,
    }
}
