use crate::{
    state::{
        Admin, LaunchPadState, LaunchPadStatus, LiquidityDetails, Lockup, Product, VestingLimits,
    },
    ProductName,
};
use concordium_cis2::TokenAmountU64 as TokenAmount;
use concordium_std::{AccountAddress, Amount, SchemaType, Serialize};

// #[derive(Serialize, SchemaType)]
// pub struct VestingView {
//     pub total_launchpad: LaunchPadID, // length of launchpad
//     pub launchpad: BTreeMap<LaunchPadID, Launchpad>,
//     pub lockup_details: BTreeMap<LaunchPadID, LockupDetails>,
// }

pub type LaunchPadsView = Vec<LaunchPadView>;

#[derive(Serialize, SchemaType, Debug)]
pub struct StateView {
    pub launch_pads: LaunchPadsView,
    pub investors: Vec<(AccountAddress, Vec<ProductName>)>,
    pub admin_info: Admin,
    pub total_launch_pads: u32,
}

#[derive(Serialize, SchemaType, Debug)]
pub struct AllLaunchPads {
    pub total_launch_pads: u32,
    pub launch_pads: Vec<LaunchPadView>,
}

#[derive(Serialize, SchemaType, Debug)]
pub struct LaunchPadView {
    pub product: ProductView,
    pub raised: Amount,
    pub status: LaunchPadStatus,
    pub holders: Vec<AccountAddress>,
    pub vest_limits: VestingLimits,
    pub soft_cap: Amount,
    pub hard_cap: Option<Amount>,
    pub allocation_paid: bool,
    pub liquidity_paid: bool,
    pub withdrawn: bool,
    pub lock_up: Lockup,
    pub liquidity_details: LiquidityDetails,
}

impl From<LaunchPadState<'_>> for LaunchPadView {
    fn from(value: LaunchPadState<'_>) -> Self {
        Self {
            product: value.product.clone().into(),
            raised: value.collected,
            status: value.status.clone(),
            holders: value.holders.iter().map(|entry| *entry.0).collect(),
            vest_limits: value.vest_limits.clone(),
            soft_cap: value.soft_cap,
            hard_cap: value.hard_cap,
            allocation_paid: value.allocation_paid,
            liquidity_paid: value.liquidity_paid,
            withdrawn: value.withdrawn,
            lock_up: value.lock_up.clone(),
            liquidity_details: value.liquidity_details.clone(),
        }
    }
}

#[derive(Serialize, SchemaType, Debug)]
pub struct ProductView {
    pub name: ProductName,
    pub owner: AccountAddress,
    pub allocated_tokens: TokenAmount,
    pub base_price: Amount,
}

impl From<Product> for ProductView {
    fn from(value: Product) -> Self {
        Self {
            name: value.name,
            owner: value.owner,
            allocated_tokens: value.allocated_tokens,
            base_price: value.token_price,
        }
    }
}
