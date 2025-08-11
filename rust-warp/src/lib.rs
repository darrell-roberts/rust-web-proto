// mod argparse;
pub mod filters;
mod handlers;
mod types;

use clap::Parser;
use std::{
    fmt::{self, Display},
    path::PathBuf,
};
use user_database::MongoArgs;

#[derive(Parser, Debug, Clone)]
#[clap(about, version, author)]
pub struct ServerOptions {
    #[clap(long)]
    pub server_cert: PathBuf,
    #[clap(long)]
    pub server_key: PathBuf,
    #[clap(flatten)]
    pub mongo_args: MongoArgs,
}

impl Display for ServerOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "server_cert {:?}, server_key {:?}, mongo_args: {})",
            self.server_cert, self.server_key, self.mongo_args
        )
    }
}
