use actix_web::{web, App, HttpServer};
use clap::Parser;
use common::USER_MS_TARGET;
use middleware::{create_test_jwt, JwtAuth};
use openssl::ssl::{SslAcceptor, SslAcceptorBuilder, SslFiletype, SslMethod};
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use tracing::{event, Level};
use tracing_actix_web::TracingLogger;
use tracing_subscriber::EnvFilter;
use types::Role;
use user_persist::mongo_persistence::MongoPersistence;
use user_persist::persistence::UserPersistence;
use user_persist::{init_mongo_client, MongoArgs};

mod common;
mod handlers;
mod middleware;
mod responders;
#[cfg(test)]
mod test;
mod types;
// mod extractors;

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

fn init_tls(args: &ProgramArgs) -> SslAcceptorBuilder {
  let mut builder =
    SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
  builder
    .set_private_key_file(args.server_tls_key_file.as_path(), SslFiletype::PEM)
    .unwrap();
  builder
    .set_certificate_chain_file(args.server_tls_cert_file.as_path())
    .unwrap();
  builder
}

#[actix_web::main]
async fn main() -> Result<(), std::io::Error> {
  tracing_subscriber::fmt()
  .with_env_filter(EnvFilter::from_default_env())
  .with_target(true)
  .pretty()
  // .json()
  // .flatten_event(true)
  .init();

  let program_opts = ProgramArgs::parse();

  let tls_opts = init_tls(&program_opts);

  event!(
    target: USER_MS_TARGET,
    Level::DEBUG,
    "Test admin jwt: {}",
    create_test_jwt(Role::Admin).unwrap()
  );

  event!(
    target: USER_MS_TARGET,
    Level::DEBUG,
    "Test user jwt: {}",
    create_test_jwt(Role::User).unwrap()
  );

  match init_mongo_client(program_opts.mongo_opts) {
    Ok(db) => {
      HttpServer::new(move || {
        let persist: web::Data<Arc<dyn UserPersistence>> =
          web::Data::new(Arc::new(MongoPersistence::new(db.clone())));
        App::new()
          .app_data(persist)
          .wrap(JwtAuth::default())
          .wrap(TracingLogger::default())
          .service(
            web::scope("/api/v1/user")
              .service(handlers::count_users)
              .service(handlers::search_users)
              .service(handlers::get_user)
              .service(handlers::save_user)
              .service(handlers::update_user),
          )
      })
      .bind_openssl("127.0.0.1:8443", tls_opts)?
      .run()
      .await
    }
    Err(e) => {
      event!(Level::ERROR, "Failed to connect to database: {}", e);
      process::exit(1);
    }
  }
}
