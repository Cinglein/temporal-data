use eyre::Context;
use std::{env, path::PathBuf, str::FromStr};
use yellowstone_grpc_proto::geyser::CommitmentLevel;

pub fn load_env(cargo_path: &str) {
    let env_path = PathBuf::from(cargo_path).join(".env");
    dotenv::from_path(env_path).ok();
}

pub fn read_env(name: &str) -> eyre::Result<String> {
    env::var(name).with_context(|| format!("Env name: {}", name))
}

pub fn read_typed_env<T>(name: &str) -> eyre::Result<T>
where
    T: FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    let string_env = read_env(name)?;
    FromStr::from_str(&string_env).map_err(Into::into)
}

const TARGET: &str = "TARGET";
const RPC: &str = "RPC";
const RPC_KEY: &str = "RPC_KEY";
const COMMITMENT: &str = "COMMITMENT";
const DATABASE_URL: &str = "DATABASE_URL";

pub struct Config {
    pub target: String,
    pub rpc: String,
    pub rpc_key: String,
    pub commitment: CommitmentLevel,
    pub database_url: String,
}

impl Config {
    pub fn load(path: &str) -> eyre::Result<Self> {
        let env_path = PathBuf::from(path).join(".env");
        dotenv::from_path(env_path).ok();
        let target = env::var(TARGET).context(TARGET)?;
        let rpc = env::var(RPC).context(RPC)?;
        let rpc_key = env::var(RPC_KEY).context(RPC_KEY)?;
        let commitment =
            CommitmentLevel::from_str_name(env::var(COMMITMENT).context(COMMITMENT)?.as_str())
                .unwrap_or_default();
        let database_url = env::var(DATABASE_URL).context(DATABASE_URL)?;
        Ok(Self {
            target,
            rpc,
            rpc_key,
            commitment,
            database_url,
        })
    }
}
