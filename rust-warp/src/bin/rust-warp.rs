// mod argparse;

use clap::Parser;
use rust_warp::{filters::user, ServerOptions};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;
use user_persist::mongo_persistence::MongoPersistence;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env())
    // .json()
    // .flatten_event(true)
    .pretty()
    .init();

  let server_args = ServerOptions::parse();

  info!("Using options: {server_args}");

  let api = user(Arc::new(
    MongoPersistence::new(server_args.mongo_args).await?,
  ));

  warp::serve(api)
    .tls()
    .cert_path(server_args.server_cert)
    .key_path(server_args.server_key)
    .run(([127, 0, 0, 1], 8443))
    .await;

  Ok(())
}
