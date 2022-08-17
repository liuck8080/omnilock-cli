use std::{fmt::format, fs, io, path::PathBuf, str::FromStr};

use ckb_types::H256;
use yaml_rust::{Yaml, YamlEmitter, YamlLoader};

pub struct ConfigContext {
    omnilock_tx_hash: H256,

    /// cell index of omnilock script deploy transaction's outputs
    omnilock_index: usize,

    /// CKB rpc url, default value = "http://127.0.0.1:8114"
    ckb_rpc: String,

    /// CKB indexer rpc url, default_value = "http://127.0.0.1:8116"
    ckb_indexer: String,
}

macro_rules! try_str {
    ($e:expr, $doc: expr) => {{
        let var = $doc[$e].as_str();
        match var {
            Some(var) => var,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("doesn't have the required item :{}", $e),
                ))
            }
        }
    }};
    ($e: expr, $doc:expr, $default_value: expr) => {{
        let var = $doc[$e].as_str();
        match var {
            Some(var) => var,
            None => $default_value,
        }
    }};
}

macro_rules! try_i64 {
    ($e:expr, $doc: expr) => {{
        let var = $doc[$e].as_i64();
        match var {
            Some(var) => var,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("doesn't have the required item :{}", $e),
                ))
            }
        }
    }};
    ($e: expr, $doc:expr, $default_value: expr) => {{
        let var = $doc[$e].as_i64();
        match var {
            Some(var) => var,
            None => $default_value,
        }
    }};
}


const TEMPLATE_CONFIG: &[u8] = include_bytes!("./config.yaml");

fn expand_home_dir(path: &str)->PathBuf {
    if path.starts_with("~") {
        let file_path = path.strip_prefix("~").unwrap();
        let file_path = if file_path.starts_with("/") {
            file_path.trim_start_matches("/")
        } else {file_path};
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
        omnilock_index: usize,
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

    pub fn parse(path: &str) -> Result<Self, io::Error> {
        let file_path = expand_home_dir(path);
        if !file_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("{} not exist!", path),
            ));
        }
        if !file_path.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{} is not a file!", path),
            ));
        }
        let content = fs::read(file_path)?;
        let content = String::from_utf8(content).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} is not a valid utf8 file:{}", path, e),
            )
        })?;

        let docs = YamlLoader::load_from_str(&content).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} is not a valid yaml file:{}", path, e),
            )
        })?;
        if docs.len() < 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Can't parse data from {}", path),
            ));
        }
        let doc = &docs[0];
        let omnilock_tx_hash = try_str!("omnilock_tx_hash", doc);
        let omnilock_tx_hash = H256::from_str(omnilock_tx_hash).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Fail to parse omnilock_tx_hash: {}", e),
            )
        })?;
        let omnilock_index = try_i64!("omnilock_index", doc) as usize;
        let ckb_rpc = try_str!("ckb_rpc", doc, "http://127.0.0.1:8114");
        let ckb_indexer = try_str!("ckb_indexer", doc, "http://127.0.0.1:8116");
        Ok(Self::new(
            omnilock_tx_hash,
            omnilock_index,
            ckb_rpc.to_string(),
            ckb_indexer.to_string(),
        ))
    }

    pub fn write_template(file_path: &str) -> Result<(), io::Error> {
        let path = expand_home_dir(file_path);
        if path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("The file or directory {} already exist!", path.as_os_str().to_str().unwrap()),
            ));
        }
        fs::write(path, TEMPLATE_CONFIG)?;
        Ok(())
    }
}
