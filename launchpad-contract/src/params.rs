use std::collections::BTreeMap;

use crate::types::*;
use concordium_std::*;

pub type Admin = AccountAddress;
#[derive(Serialize, SchemaType)]
pub struct InitParameter {
    pub admin: Admin,
}

#[derive(Serialize, SchemaType)]
pub struct LivePauseParam {
    pub id: LaunchpadID,
    pub is_live: Live,
}

#[derive(Serial, Deserial, SchemaType)]
pub struct ClaimParams {
    pub id: ContractTokenId,
    pub address: ContractAddress,
    pub launchpad_id: u64,
    pub epoch_cycle: u8,
}

#[derive(Serial, Deserial, SchemaType)]
pub struct TokenParam {
    pub id: ContractTokenId,
    pub address: ContractAddress,
    pub token_amount: ContractTokenAmount,
}

#[derive(Serialize, SchemaType)]
pub struct LaunchpadParam {
    pub start_time: Timestamp,
    pub end_time: Timestamp,
    pub live: bool,
    pub soft_cap: u64,
    pub hard_cap: u64,
    pub minimum_invest: u64,
    pub maximum_invest: u64,
    pub cliff_duration: u64,
    pub token_param: TokenParam,
    pub token_release_data: BTreeMap<ReleaseCycles, ReleaseData>,
    pub cis2_price: u32,
}

#[derive(Serialize, SchemaType)]
pub struct VestParams {
    pub launchpad_id: LaunchpadID,
    pub token_amount: u64,
}

#[derive(Serialize, SchemaType, Clone)]
pub struct TokenInfo {
    pub id: ContractTokenId,
    pub address: ContractAddress,
}
#[derive(Serialize, SchemaType, Clone)]
pub struct CancelParam {
    pub launchpad_id: LaunchpadID,
    pub token: TokenInfo,
}

#[derive(Serialize, SchemaType, Clone)]
pub struct WithdrawParam {
    pub launchpad_id: LaunchpadID,
    pub token: TokenInfo,
    pub remaining_cis2_amount: ContractTokenAmount,
}
