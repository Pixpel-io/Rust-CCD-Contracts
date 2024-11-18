use crate::errors::VestingError;
use concordium_cis2::*;
use concordium_std::{collections::BTreeMap, *};

/// Number of holder.
pub type TotalHolders = u32;
/// single holder amount
pub type TotalHolderAmount = u64;
// its and id and total launchpad at same time
pub type LaunchpadID = u64;

pub type ReleaseCycles = u8;

pub type UserBalance = Amount;
pub type User = AccountAddress;
pub type Live = bool;

pub type ContractTokenId = TokenIdU8;

pub type ContractTokenAmount = TokenAmountU64;

pub type ContractResult<A> = Result<A, VestingError>;

#[derive(Serial, Deserial, SchemaType, Clone, Debug)]
pub struct ReleaseData {
    pub release_time: Timestamp,
    pub per_cycle_release: u64,
}

// pub type TransferParameter(T,A) = TransferParams<T: IsTokenId, A: IsTokenAmount>;

#[derive(Serialize, SchemaType, Clone)]
pub struct Launchpad {
    pub start_time: Timestamp,
    pub end_time: Timestamp,
    pub live: bool,
    pub pause_until: Timestamp,
    pub pause_start: Timestamp,
    pub live_pause_count: u8,
    pub cancel: bool,
    pub soft_cap: Amount,
    pub hard_cap: Amount,
    pub invest_amount: Amount,
    pub holders: BTreeMap<AccountAddress, Amount>,
    pub owner: AccountAddress,
    pub minimum_invest: Amount,
    pub maximum_invest: Amount,
    pub total_tx: u64,
    pub dev_paid: Amount,
    pub cis2_amount: TokenAmountU64,
    pub cis2_price: u32,
}
#[derive(Serial, Deserial, SchemaType, Clone, Debug)]
pub struct LockupHolder {
    pub cycle_completed: u8,
    pub claimable_token: u64,
    pub vested_date: Timestamp,
}

#[derive(Serialize, SchemaType, Clone)]
pub struct LockupDetails {
    pub cliff_period: Timestamp,
    pub token_release_data: BTreeMap<ReleaseCycles, ReleaseData>,
    pub lockup_holders: BTreeMap<AccountAddress, LockupHolder>,
    pub cliff_duration: u64,
}

#[derive(Serialize, Clone)]
struct State {
    total_launchpad: LaunchpadID, // length of launchpad
    launchpad: BTreeMap<LaunchpadID, Launchpad>,
    admin: AccountAddress,
    lockup_details: BTreeMap<LaunchpadID, LockupDetails>,
}

pub type VestingResult<T> = Result<T, VestingError>;

// Contract functions
