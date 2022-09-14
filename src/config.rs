use std::{fs, path::PathBuf, str::FromStr};

use anyhow::{anyhow, ensure, Context, Result};
use ckb_types::H256;
use clap::Subcommand;
use yaml_rust::YamlLoader;

use crate::client::build_omnilock_cell_dep;

#[derive(Subcommand)]
pub(crate) enum ConfigCmds {
    /// Init an empty configure file
    Init,
    /// Check if the configure file correct
    Check,
}
pub(crate) fn handle_config_cmds(cmds: &ConfigCmds, path: &str) -> Result<()> {
    match cmds {
        ConfigCmds::Init => {
            ConfigContext::write_template(path).map(|_| {
                println!(
                    "The template file {} generated, please fill it with the correct content.",
                    path
                );
            })?;
        }
        ConfigCmds::Check => {
            ConfigContext::check(path)?;
        }
    };
    Ok(())
}

pub struct ConfigContext {
    pub omnilock_tx_hash: H256,

    /// cell index of omnilock script deploy transaction's outputs
    pub omnilock_index: u32,

    /// CKB rpc url
    pub ckb_rpc: String,

    /// CKB indexer rpc url
    pub ckb_indexer: String,
}

macro_rules! try_str {
    ($e:expr, $doc: expr) => {
        $doc[$e]
            .as_str()
            .ok_or(anyhow!("doesn't have the required item :{}", $e))?
    };
}

macro_rules! try_i64 {
    ($e:expr, $doc: expr) => {{
        $doc[$e]
            .as_i64()
            .ok_or(anyhow!("doesn't have the required item :{}", $e))?
    }};
}

const TEMPLATE_CONFIG: &[u8] = include_bytes!("./config.yaml");

fn expand_home_dir(path: &str) -> PathBuf {
    if path.starts_with('~') {
        let file_path = path.strip_prefix('~').unwrap();
        let file_path = if file_path.starts_with('/') {
            file_path.trim_start_matches('/')
        } else {
            file_path
        };
        let mut dir = dirs::home_dir().unwrap();
        dir.push(file_path);
        dir
    } else {
        PathBuf::from(path)
    }
}

impl ConfigContext {
    pub fn new(
        omnilock_tx_hash: H256,
        omnilock_index: u32,
        ckb_rpc: String,
        ckb_indexer: String,
    ) -> Self {
        ConfigContext {
            omnilock_tx_hash,
            omnilock_index,
            ckb_rpc,
            ckb_indexer,
        }
    }

    pub fn parse(path: &str) -> Result<Self> {
        let file_path = expand_home_dir(path);
        ensure!(file_path.exists(), "{} not exist!", path);
        ensure!(file_path.is_file(), "{} is not a file!", path);
        let content = fs::read(file_path)?;
        let content = String::from_utf8(content)
            .with_context(|| format!("{} is not a valid utf8 file.", path))?;

        let docs = YamlLoader::load_from_str(&content)
            .with_context(|| format!("{} is not a valid yaml file", path))?;
        ensure!(!docs.is_empty(), "Can't parse data from {}", path);
        let doc = &docs[0];
        let omnilock_tx_hash = try_str!("omnilock_tx_hash", doc);
        let omnilock_tx_hash =
            H256::from_str(omnilock_tx_hash).with_context(|| "Fail to parse omnilock_tx_hash")?;
        let omnilock_index = try_i64!("omnilock_index", doc);
        let omnilock_index = u32::try_from(omnilock_index)?;
        let ckb_rpc = try_str!("ckb_rpc", doc);
        let ckb_indexer = try_str!("ckb_indexer", doc);
        Ok(Self::new(
            omnilock_tx_hash,
            omnilock_index,
            ckb_rpc.to_string(),
            ckb_indexer.to_string(),
        ))
    }

    pub fn write_template(file_path: &str) -> Result<()> {
        let path = expand_home_dir(file_path);
        ensure!(
            !path.exists(),
            "The exist {} should not be overwrited",
            file_path
        );
        fs::write(path, TEMPLATE_CONFIG)?;
        Ok(())
    }

    pub fn check(path: &str) -> Result<()> {
        let env = Self::parse(path)?;

        build_omnilock_cell_dep(
            env.ckb_rpc.as_str(),
            &env.omnilock_tx_hash,
            env.omnilock_index,
        )?;
        println!("the configure file `{0}` is ok!", path);
        Ok(())
    }
}
