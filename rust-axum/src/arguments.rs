//! Program arguments and application state.
use crate::{JWTClaims, Role};
use chrono::{Duration, Utc};
use clap::Parser;
use jsonwebtoken::{encode, DecodingKey, EncodingKey, Header};
use std::path::PathBuf;
use user_persist::MongoArgs;

/// Command line arguments.
#[derive(Parser, Clone)]
#[clap(about, version, author)]
pub struct ProgramArgs {
    #[clap(flatten)]
    pub mongo_opts: MongoArgs,
    #[clap(long)]
    #[clap(help = "ssl tls key file")]
    pub server_tls_key_file: PathBuf,
    #[clap(long)]
    #[clap(help = "ssl tls certificate file")]
    pub server_tls_cert_file: PathBuf,
    #[clap(long)]
    #[clap(help = "JWT Secret")]
    pub jwt_secret: String,
}

impl ProgramArgs {
    pub fn server_tls_key_file(&self) -> &PathBuf {
        &self.server_tls_key_file
    }

    pub fn server_tls_cert_file(&self) -> &PathBuf {
        &self.server_tls_cert_file
    }

    pub fn mongo_opts(self) -> MongoArgs {
        self.mongo_opts
    }
}

/// Application State.
#[derive(Clone)]
pub struct AppConfig {
    jwt_encoding_key: EncodingKey,
    jwt_decoding_key: DecodingKey,
    hash_prefix: String,
}

impl AppConfig {
    /// Create a new application config state.
    pub fn new(secret: &[u8]) -> Self {
        Self {
            jwt_decoding_key: DecodingKey::from_secret(secret),
            jwt_encoding_key: EncodingKey::from_secret(secret),
            hash_prefix: "some_secret_prefix".to_owned(),
        }
    }

    /// Get a reference to the JWT encoding key.
    pub fn jwt_encoding_key(&self) -> &EncodingKey {
        &self.jwt_encoding_key
    }

    /// Get a reference to the JWT decoding key.
    pub fn jwt_decoding_key(&self) -> &DecodingKey {
        &self.jwt_decoding_key
    }

    /// Get a reference to the prefix for hashing.
    pub fn hash_prefix(&self) -> &str {
        &self.hash_prefix
    }
}

/// Creates a test JWT for the given role.
pub fn test_jwt(opts: &AppConfig, role: Role) -> String {
    let expiration = Utc::now() + Duration::minutes(25);
    let test_claims = JWTClaims {
        sub: "droberts".to_owned(),
        role,
        exp: expiration.timestamp(),
    };
    encode(&Header::default(), &test_claims, &opts.jwt_encoding_key).unwrap()
}
