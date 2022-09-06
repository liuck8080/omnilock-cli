use ckb_jsonrpc_types as json_types;
use ckb_sdk::unlock::OmniLockConfig;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct TxInfo {
    pub transaction: json_types::Transaction,
    pub omnilock_config: OmniLockConfig,
}
