use anyhow::{Context, Result};
use ckb_sdk::{CkbRpcClient, ScriptId};
use ckb_types::{
    packed::{Byte32, CellDep, OutPoint, Script},
    prelude::*,
    H256,
};

#[allow(dead_code)]
pub struct OmniLockInfo {
    pub type_hash: H256,
    pub script_id: ScriptId,
    pub cell_dep: CellDep,
}

pub fn build_omnilock_cell_dep(
    ckb_client: &mut CkbRpcClient,
    tx_hash: &H256,
    index: u32,
) -> Result<OmniLockInfo> {
    let out_point_json = ckb_jsonrpc_types::OutPoint {
        tx_hash: tx_hash.clone(),
        index: ckb_jsonrpc_types::Uint32::from(index as u32),
    };
    let cell_status = ckb_client
        .get_live_cell(out_point_json, false)
        .with_context(|| "while try to load live cells".to_string())?;
    let script = Script::from(cell_status.cell.unwrap().output.type_.unwrap());

    let type_hash = script.calc_script_hash();
    let out_point = OutPoint::new(Byte32::from_slice(tx_hash.as_bytes())?, index as u32);

    let cell_dep = CellDep::new_builder().out_point(out_point).build();
    Ok(OmniLockInfo {
        type_hash: H256::from_slice(type_hash.as_slice())?,
        script_id: ScriptId::new_type(type_hash.unpack()),
        cell_dep,
    })
}
