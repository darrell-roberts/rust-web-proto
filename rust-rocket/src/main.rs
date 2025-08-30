//! Start the user service from cli.
use clap::Parser;
use rust_rocket::{
    test_jwt,
    types::{self, Role},
};
use std::{fmt, process, sync::Arc};
use tracing::{error, event, Level};
use tracing_subscriber::EnvFilter;
use user_database::{database::UserDatabaseDynSafe, mongo_database::MongoDatabase, MongoArgs};

#[derive(Parser, Debug, Clone)]
#[clap(about, version, author)]
struct ProgramArgs {
    #[clap(flatten)]
    mongo_opts: MongoArgs,
}

impl fmt::Display for ProgramArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mongo_opts {}", self.mongo_opts)
    }
}

#[rocket::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(true)
        .pretty()
        // .json()
        // .flatten_event(true)
        .init();

    let program_opts = ProgramArgs::parse();

    event!(
      target: types::USER_MS_TARGET,
      Level::INFO,
      "mongo_args: {program_opts}"
    );

    event!(
      target: types::USER_MS_TARGET,
      Level::DEBUG,
      "admin {}",
      test_jwt(Role::Admin)
    );

    match MongoDatabase::new(program_opts.mongo_opts).await {
        Ok(db) => {
            let mongo_database: Arc<dyn UserDatabaseDynSafe> = Arc::new(db);
            let _ = rust_rocket::rocket(mongo_database).launch().await.unwrap();
        }
        Err(e) => {
            error!("Failed to connect to database: {e}");
            process::exit(1);
        }
    };
}
