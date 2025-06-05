use crate::{
    state::{Admin, LiquidityDetails, Product, TimePeriod, VestingLimits, DAYS},
    ProductName,
};
use concordium_cis2::TokenAmountU64 as TokenAmount;
use concordium_std::{Amount, Deserial, Duration, SchemaType, Serial, Serialize, Timestamp};

pub type Months = u64;

#[derive(Serialize, SchemaType)]
pub struct InitParams {
    pub admin: Admin,
}

#[derive(Serialize, SchemaType, Clone, Debug)] // Added Clone
pub struct CreateParams {
    pub product: Product,
    pub timeperiod: TimePeriod,
    pub soft_cap: Amount,
    pub hard_cap: Option<Amount>,
    pub vest_limits: VestingLimits,
    pub lockup_details: LockupDetails,
    pub liquidity_details: LiquidityDetails,
}

impl CreateParams {
    pub fn cliff(&self) -> Duration {
        Duration::from_days((self.lockup_details.cliff * DAYS) as u64)
    }

    pub fn launchpad_end_time(&self) -> Timestamp {
        self.timeperiod.end
    }
}

#[derive(Serialize, SchemaType, Clone, Debug)] // Added Debug
pub struct LockupDetails {
    pub cliff: Months,
    pub release_cycles: Months,
}

#[derive(Serialize, SchemaType)]
pub struct ApprovalParams {
    pub product_name: ProductName,
    pub approve: bool,
}

#[derive(Serial, Deserial, SchemaType, Debug)]
pub struct LivePauseParams {
    pub poduct_name: ProductName,
    pub pause_duration: TimePeriod,
    pub to_pause: bool,
}

#[derive(Serialize, SchemaType, Debug)]
pub struct VestParams {
    pub product_name: ProductName,
    pub token_amount: TokenAmount,
}

#[derive(Serial, Deserial, SchemaType, Debug, Clone)]
pub enum Claimer {
    OWNER(u8),
    HOLDER(u8),
}

#[derive(Serial, Deserial, SchemaType, Debug, Clone)]
pub struct ClaimLockedParams {
    pub claimer: Claimer,
    pub product_name: ProductName,
}

#[derive(Serial, Deserial, SchemaType)]
pub struct ClaimUnLockedParams {
    pub cycle: u8,
    pub product_name: ProductName,
}
