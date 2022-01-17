use axum::{
  routing::{get, post, put},
  AddExtensionLayer, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use http::header::HeaderName;
use std::net::SocketAddr;
use std::{path::PathBuf, sync::Arc};
use tower::ServiceBuilder;
use tower_http::{
  compression::CompressionLayer, propagate_header::PropagateHeaderLayer,
  request_id::SetRequestIdLayer, trace::TraceLayer,
};
use tracing::{event, Level};
use tracing_subscriber::EnvFilter;
use user_persist::mongo_persistence::MongoPersistence;
use user_persist::persistence::UserPersistence;
use user_persist::{init_mongo_client, MongoArgs};

mod extractors;
mod handlers;
mod middleware;
mod types;

pub const USER_MS_TARGET: &str = "user-ms";

#[derive(Parser, Debug, Clone)]
#[clap(about, version, author)]
struct ProgramArgs {
  #[clap(flatten)]
  mongo_opts: MongoArgs,
  #[clap(long)]
  server_tls_key_file: PathBuf,
  #[clap(long)]
  server_tls_cert_file: PathBuf,
}

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt()
  .with_env_filter(EnvFilter::from_default_env())
  .with_target(true)
  .pretty()
  // .json()
  // .flatten_event(true)
  .init();

  let program_opts = ProgramArgs::parse();
  let x_request_id = HeaderName::from_static("x-request-id");

  match init_mongo_client(program_opts.mongo_opts) {
    Ok(db) => {
      let persist: Arc<dyn UserPersistence> =
        Arc::new(MongoPersistence::new(db.clone()));

      let user_routes = Router::new()
        .route("/user/:id", get(handlers::get_user))
        .route("/user", post(handlers::save_user))
        .route("/user", put(handlers::update_user))
        .route("/user/search", post(handlers::search_users))
        .route("/user/counts", get(handlers::count_users));

      let tower_middleware = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(SetRequestIdLayer::new(
          x_request_id.clone(),
          middleware::MakeRequestUuid,
        ))
        .layer(PropagateHeaderLayer::new(x_request_id))
        .layer(AddExtensionLayer::new(persist))
        .layer(CompressionLayer::new());

      let app = Router::new()
        .nest("/api/v1", user_routes)
        .layer(tower_middleware);

      let addr = SocketAddr::from(([127, 0, 0, 1], 8443));

      let config = RustlsConfig::from_pem_file(
        program_opts.server_tls_cert_file,
        program_opts.server_tls_key_file,
      )
      .await
      .unwrap();

      axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .unwrap();
    }
    Err(e) => {
      event!(Level::ERROR, "Failed to initialize monogdb: {e}");
    }
  }
}
