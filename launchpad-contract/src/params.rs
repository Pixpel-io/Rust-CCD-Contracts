use std::collections::BTreeMap;

use crate::{state::{Admin, Product, TimePeriod, VestingLimits}, types::*};
use concordium_std::*;

/// Contract initialization parameters to be passed at the time
/// of contract init.
/// 
/// Launch-pad contract is initialized with the provided admin
/// details.
#[derive(Serialize, SchemaType)]
pub struct InitParams {
    /// Admin details such as admin account address,
    /// registeration fee, soft-cap share
    pub admin: Admin,
}

/// Parameters to be passed while invoking the `CreateLaunchPad` by user
#[derive(Serialize, SchemaType)]
pub struct CreateParams {
    /// Details regarding the product being listed for presale
    pub product: Product,
    /// Time duration for the presale
    pub timeperiod: TimePeriod,
    /// Bare minimum funds to be raised for a presale to be successful
    pub soft_cap: Amount,
    /// Optional surplus funds to be raised for a presale to be successful
    pub hard_cap: Option<Amount>,
    /// Defines the maximum and minimum investment amounts acceptable
    /// for presale
    pub vest_limits: VestingLimits,
    /// Lock up information for vesting releases
    pub lockup_details: LockupDetails
}

impl CreateParams {
    /// Getter function to get the provided timestamp
    /// of cliff
    pub fn cliff_timestamp(&self) -> Timestamp {
        self.lockup_details.cliff
    }

    /// Getter function to get the provided ending time
    /// of current launch-pad
    pub fn launchpad_end_time(&self) -> Timestamp {
        self.timeperiod.end
    }
}

/// Lock up information to be provided by the user in `CreateLaunchPad`
#[derive(Serialize, SchemaType)]
pub struct LockupDetails {
    /// Cliff duration until vesting starts
    pub cliff: Timestamp,
    /// Vesting cycles based on months for linear vesting 
    pub release_cycles: u8,
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
