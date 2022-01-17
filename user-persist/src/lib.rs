pub mod mongo_persistence;
pub mod persistence;
pub mod types;

use clap::Args;
use mongodb::options::{
  AuthMechanism, ClientOptions, Credential, ServerAddress, Tls, TlsOptions,
};
use mongodb::Client;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

pub const PERSISTENCE_TARGET: &str = "persistence";

// Setup mongodb client. This setup uses TLS with cert and ca file and
// credentials.
pub fn init_mongo_client(
  args: MongoArgs,
) -> Result<mongodb::Database, mongodb::error::Error> {
  let db_name = &args.mongo_db.clone();

  let credentials = Credential::builder()
    .username(Some(args.mongo_user))
    .password(Some(args.mongo_pass))
    .source(Some(args.mongo_db))
    .mechanism(Some(AuthMechanism::ScramSha256))
    .build();

  let tls_options = TlsOptions::builder()
    .allow_invalid_certificates(Some(true))
    .ca_file_path(Some(args.mongo_ca_file))
    .cert_key_file_path(Some(args.mongo_key_file))
    .build();

  let mongo_options = ClientOptions::builder()
    .hosts(vec![args.mongo_host])
    .tls(Some(Tls::Enabled(tls_options)))
    .app_name(args.app_name)
    .direct_connection(true)
    .credential(credentials)
    .build();

  Client::with_options(mongo_options).map(|c| c.database(db_name))
}

#[derive(Args, Debug, Clone)]
#[clap(about, version, author)]
pub struct MongoArgs {
  #[clap(long)]
  pub mongo_user: String,
  #[clap(long)]
  pub mongo_pass: String,
  #[clap(long)]
  pub mongo_db: String,
  #[clap(long)]
  pub mongo_host: ServerAddress,
  #[clap(long)]
  pub app_name: String,
  #[clap(long)]
  pub mongo_ca_file: PathBuf,
  #[clap(long)]
  pub mongo_key_file: PathBuf,
}

impl Display for MongoArgs {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "mongo_user ***** \
      mongo_pass ***** \
      mongo_db {} \
      mongo_host {} \
      app_name {} \
      mongo_ca_file {:?} \
      mongo_key_file {:?} \
      ",
      self.mongo_db,
      self.mongo_host,
      self.app_name,
      self.mongo_ca_file,
      self.mongo_key_file,
    )
  }
}
