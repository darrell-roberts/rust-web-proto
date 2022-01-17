// mod argparse;
mod filters;
mod handlers;
#[cfg(test)]
mod test;
mod types;

use clap::Parser;
use filters::user;
use std::fmt::{self, Display};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;
use user_persist::mongo_persistence::MongoPersistence;
use user_persist::{init_mongo_client, MongoArgs};

#[derive(Parser, Debug, Clone)]
#[clap(about, version, author)]
struct ServerOptions {
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

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env())
    // .json()
    // .flatten_event(true)
    .pretty()
    .init();

  let server_args = ServerOptions::parse();

  info!("Using options: {server_args}");
  let mongo_db = init_mongo_client(server_args.mongo_args).unwrap();

  let api = user(Arc::new(MongoPersistence::new(mongo_db)));

  warp::serve(api)
    .tls()
    .cert_path(server_args.server_cert)
    .key_path(server_args.server_key)
    .run(([127, 0, 0, 1], 8443))
    .await;
}
