use clap::Parser;
use openssl::ssl::{SslAcceptor, SslAcceptorBuilder, SslFiletype, SslMethod};
use std::path::PathBuf;
use user_persist::MongoArgs;

pub mod common;
pub mod handlers;
pub mod middleware;
mod responders;
pub mod types;

#[derive(Parser, Debug, Clone)]
#[clap(about, version, author)]
pub struct ProgramArgs {
    #[clap(flatten)]
    pub mongo_opts: MongoArgs,
    #[clap(long)]
    server_tls_key_file: PathBuf,
    #[clap(long)]
    server_tls_cert_file: PathBuf,
}

pub fn init_tls(args: &ProgramArgs) -> SslAcceptorBuilder {
    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder
        .set_private_key_file(args.server_tls_key_file.as_path(), SslFiletype::PEM)
        .unwrap();
    builder
        .set_certificate_chain_file(args.server_tls_cert_file.as_path())
        .unwrap();
    builder
}
