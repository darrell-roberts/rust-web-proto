use axum::extract::Extension;
use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use rust_axum::{
  arguments::{test_jwt, AppConfig, ProgramArgs},
  build_app,
  types::jwt::Role,
  USER_MS_TARGET,
};
use std::{error::Error, net::SocketAddr, sync::Arc};
use tracing::{event, Level};
use tracing_subscriber::EnvFilter;
use user_persist::mongo_persistence::MongoPersistence;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env())
    .with_target(true)
    .pretty()
    // .json()
    // .flatten_event(true)
    .init();

  let program_opts = ProgramArgs::parse();
  let app_config = AppConfig::new(&program_opts);

  // Print out some test JWT's.
  event!(
    target: USER_MS_TARGET,
    Level::DEBUG,
    "test admin jwt: {}",
    test_jwt(&app_config, Role::Admin)
  );

  event!(
    target: USER_MS_TARGET,
    Level::DEBUG,
    "test user jwt: {}",
    test_jwt(&app_config, Role::User)
  );

  let config = RustlsConfig::from_pem_file(
    program_opts.server_tls_cert_file(),
    program_opts.server_tls_key_file(),
  )
  .await?;

  let mongo_persist =
    Arc::new(MongoPersistence::new(program_opts.mongo_opts()).await?);

  let app = build_app(mongo_persist.clone(), app_config)
    .layer(Extension(mongo_persist));

  let addr = SocketAddr::from(([0, 0, 0, 0], 8443));
  axum_server::bind_rustls(addr, config)
    .serve(app.into_make_service())
    .await
    .map(Ok)?
}
