use crate::{state::{Admin, LiquidityDetails, Product, TimePeriod, VestingLimits, DAYS}, ProductName};
use concordium_cis2::{TokenAmountU64 as TokenAmount, TokenIdVec};
use concordium_std::*;

pub type Months = u64;
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
    pub lockup_details: LockupDetails,
    /// Token Liquidity information to lock the funds
    pub liquidity_details: LiquidityDetails
}

impl CreateParams {
    /// Getter function to get the provided time period
    /// of cliff
    pub fn cliff(&self) -> Duration {
        Duration::from_days((self.lockup_details.cliff * DAYS) as u64)
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
    pub cliff: Months,
    /// Vesting cycles based on months for linear vesting 
    pub release_cycles: Months,
}

/// Parameters to be passed while invoking the `ApproveLaunchPad` by admin
/// to approve or reject the Launch-pad
#[derive(Serialize, SchemaType)]
pub struct ApprovalParams {
    /// Product name to uniquely identify the launch-pad
    /// for approval
    pub product_name: ProductName,
    /// A boolean if `true` means approved, if `false`
    /// mean rejected 
    pub approve: bool,
}

/// Parameters to be passed while invoking `LivePause` to pause or resume 
/// launch-pad vesting
#[derive(Serialize, SchemaType)]
pub struct LivePauseParams {
    /// Product name for unique launch-pad identification
    pub poduct_name: ProductName,
    /// Duration for which the launch-pad is to be pause.
    /// It must be greater than 48 hrs
    pub pause_duration: TimePeriod,
    /// Boolean for making launch pause or live
    pub to_pause: bool,
}

/// Parameters to be passed while invoking `Vest` to invest on a launch pad 
#[derive(Serialize, SchemaType)]
pub struct VestParams {
    /// Product name to identify launch pad in contract
    /// state
    pub product_name: ProductName,
    /// Amount of token to be bought from allocation
    /// in presale
    pub token_amount: TokenAmount,
}

#[derive(Serial, Deserial, SchemaType)]
pub struct AddLiquidityParams {
    pub token: TokenInfo,
    pub token_amount: TokenAmount,
}

#[derive(Serial, Deserial, SchemaType, Clone, Debug)]
pub struct TokenInfo {
    pub id: TokenIdVec,
    pub address: ContractAddress,
}

#[derive(Serial, Deserial, SchemaType)]
pub struct GetExchangeParams {
    pub holder: Address,
    pub token: TokenInfo,
}

#[derive(Serial, Deserial, SchemaType)]
pub enum Claimer {
    OWNER(u8),
    HOLDER(u8)
}

#[derive(Serial, Deserial, SchemaType)]
pub struct ClaimLockedParams {
    pub claimer: Claimer,
    pub product_name: ProductName
}

#[derive(Serial, Deserial, SchemaType)]
pub struct ClaimUnLockedParams {
    pub cycle: u8,
    pub product_name: ProductName
}

// #[derive(Serial, Deserial, SchemaType)]
// pub struct ClaimTokenParams {
//     pub id: TokenID,
//     pub address: ContractAddress,
//     pub launchpad_id: u64,
//     pub epoch_cycle: u8,
// }

// #[derive(Serial, Deserial, SchemaType)]
// pub struct TokenParam {
//     pub id: TokenID,
//     pub address: ContractAddress,
//     pub token_amount: TokenAmount,
// }

// #[derive(Serialize, SchemaType)]
// pub struct LaunchpadParam {
//     pub start_time: Timestamp,
//     pub end_time: Timestamp,
//     pub live: bool,
//     pub soft_cap: u64,
//     pub hard_cap: u64,
//     pub minimum_invest: u64,
//     pub maximum_invest: u64,
//     pub cliff_duration: u64,
//     pub token_param: TokenParam,
//     pub token_release_data: BTreeMap<ReleaseCycles, ReleaseData>,
//     pub cis2_price: u32,
// }



// #[derive(Serialize, SchemaType, Clone)]
// pub struct TokenInfo {
//     pub id: TokenID,
//     pub address: ContractAddress,
// }
// #[derive(Serialize, SchemaType, Clone)]
// pub struct CancelParam {
//     pub launchpad_id: LaunchPadID,
//     pub token: TokenInfo,
// }

// #[derive(Serialize, SchemaType, Clone)]
// pub struct WithdrawParam {
//     pub launchpad_id: LaunchPadID,
//     pub token: TokenInfo,
//     pub remaining_cis2_amount: TokenAmount,
// }
