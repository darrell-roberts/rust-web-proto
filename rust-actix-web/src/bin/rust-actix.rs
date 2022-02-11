use actix_web::{web, App, HttpServer};
use clap::Parser;
use rust_actix_web::{
  common::USER_MS_TARGET,
  handlers, init_tls,
  middleware::{create_test_jwt, JwtAuth},
  types::Role,
  ProgramArgs,
};
use std::{process, sync::Arc};
use tracing::{event, Level};
use tracing_actix_web::TracingLogger;
use tracing_subscriber::EnvFilter;
use user_persist::{
  mongo_persistence::MongoPersistence, persistence::UserPersistence,
};

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

  match MongoPersistence::new(program_opts.mongo_opts).await {
    Ok(persistence) => {
      HttpServer::new(move || {
        let persist: web::Data<Arc<dyn UserPersistence>> =
          web::Data::new(Arc::new(persistence.clone()));
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
