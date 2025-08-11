//! Command line interface for starting the User REST API server.
use axum::extract::Extension;
use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use rust_axum::{
    arguments::{AppConfig, ProgramArgs},
    build_app,
};
use std::{error::Error, net::SocketAddr, sync::Arc};
use tracing_subscriber::EnvFilter;
use user_database::mongo_database::MongoDatabase;

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
    let app_config = AppConfig::new(program_opts.jwt_secret.as_bytes());

    let config = RustlsConfig::from_pem_file(
        program_opts.server_tls_cert_file(),
        program_opts.server_tls_key_file(),
    )
    .await?;

    let database = Arc::new(MongoDatabase::new(program_opts.mongo_opts()).await?);

    let app = build_app(database.clone(), app_config).layer(Extension(database));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8443));
    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .map(Ok)?
}
