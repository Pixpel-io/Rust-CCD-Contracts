use crate::{
    params::TokenInfo,
    state::{
        Admin, HolderInfo, LaunchPadState, LaunchPadStatus, LiquidityDetails, Lockup, Product,
        VestingLimits,
    },
    ProductName,
};
use concordium_cis2::{TokenAmountU64 as TokenAmount, TokenIdU64};
use concordium_std::{
    schema::{self, SchemaType},
    AccountAddress, Amount, Deserial, SchemaType, Serial, Serialize, StateRef, Timestamp,
};

// #[derive(Serialize, SchemaType)]
// pub struct VestingView {
//     pub total_launchpad: LaunchPadID, // length of launchpad
//     pub launchpad: BTreeMap<LaunchPadID, Launchpad>,
//     pub lockup_details: BTreeMap<LaunchPadID, LockupDetails>,
// }

pub type LaunchPadsView = Vec<LaunchPadView>;

#[derive(Serial, Deserial, SchemaType, Debug)]
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
    // #[derivative(Debug(format_with = "debug_amount"))]
    pub raised: Amount,
    pub status: LaunchPadStatus,
    // #[derivative(Debug(format_with = "debug_holders"))]
    pub holders: Vec<(AccountAddress, HolderView)>,
    pub vest_limits: VestingLimits,
    // #[derivative(Debug(format_with = "debug_amount"))]
    pub soft_cap: Amount,
    // #[derivative(Debug(format_with = "debug_optional_amount"))]
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
            holders: value
                .holders
                .iter()
                .map(|entry| {
                    (
                        *entry.0,
                        <StateRef<'_, HolderInfo> as Into<HolderView>>::into(entry.1),
                    )
                })
                .collect(),
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
    // #[derivative(Debug(format_with = "account_address"))]
    pub owner: AccountAddress,
    // #[derivative(Debug(format_with = "debug_tokens_amount"))]
    pub allocated_tokens: TokenAmount,
    // #[derivative(Debug(format_with = "debug_amount"))]
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

#[derive(Serialize, SchemaType, Debug)]
pub struct HolderView {
    // #[derivative(Debug(format_with = "debug_tokens_amount"))]
    pub tokens: TokenAmount,
    // #[derivative(Debug(format_with = "debug_amount"))]
    pub invested: Amount,
    pub unlocked_release: Vec<(u8, UnlockedWrapper)>,
    pub locked_release: Vec<(u8, LockedWrapper)>,
}

impl From<StateRef<'_, HolderInfo>> for HolderView {
    fn from(value: StateRef<'_, HolderInfo>) -> Self {
        Self {
            tokens: value.tokens,
            invested: value.invested,
            unlocked_release: value
                .release_data
                .unlocked
                .iter()
                .map(|details| (*details.0, (*details.1).into()))
                .collect(),
            locked_release: value
                .release_data
                .locked
                .iter()
                .map(|details| (*details.0, (*details.1).into()))
                .collect(),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct UnlockedWrapper(pub (TokenAmount, Timestamp, bool));

impl SchemaType for UnlockedWrapper {
    fn get_type() -> crate::schema::Type {
        let fields = schema::Fields::Unnamed(vec![
            TokenAmount::get_type(),
            Timestamp::get_type(),
            bool::get_type(),
        ]);

        schema::Type::Struct(fields)
    }
}

impl From<(TokenAmount, Timestamp, bool)> for UnlockedWrapper {
    fn from(value: (TokenAmount, Timestamp, bool)) -> Self {
        Self(value)
    }
}

#[derive(Serialize, Debug)]
pub struct LockedWrapper(pub (TokenAmount, TokenIdU64, Timestamp, bool));

impl SchemaType for LockedWrapper {
    fn get_type() -> crate::schema::Type {
        let fields = schema::Fields::Unnamed(vec![
            TokenAmount::get_type(),
            TokenIdU64::get_type(),
            Timestamp::get_type(),
            bool::get_type(),
        ]);

        schema::Type::Struct(fields)
    }
}

impl From<(TokenAmount, TokenIdU64, Timestamp, bool)> for LockedWrapper {
    fn from(value: (TokenAmount, TokenIdU64, Timestamp, bool)) -> Self {
        Self(value)
    }
}

#[derive(Serialize, SchemaType, Debug)]
pub struct ExchangeView {
    pub token: TokenInfo,
    pub token_balance: TokenAmount,
    pub ccd_balance: TokenAmount,
    pub lp_token_id: TokenIdU64,
    pub lp_tokens_supply: TokenAmount,
    pub lp_tokens_holder_balance: TokenAmount,
}

// fn account_address(address: &AccountAddress, f: &mut concordium_std::fmt::Formatter) -> std::fmt::Result {
//     write!(f, "AccountAddress({:?})", address.to_string())
// }

// fn debug_holders(
//     holders: &Vec<(AccountAddress, HolderView)>,
//     f: &mut concordium_std::fmt::Formatter,
// ) -> std::fmt::Result {
//     writeln!(f, "[")?;
//     for holder in holders {
//         writeln!(f, "\tAccountAddress({:?}),", holder.0.to_string())?;
//         writeln!(f, "\t{:#?}", holder.1)?
//     }
//     writeln!(f, "]")?;
//     Ok(())
// }

// pub fn debug_amount(amount: &Amount, f: &mut concordium_std::fmt::Formatter) -> std::fmt::Result {
//     write!(f, "{} CCD", amount.micro_ccd / 1000000)
// }

// fn debug_optional_amount(amount: &Option<Amount>, f: &mut concordium_std::fmt::Formatter) -> std::fmt::Result {
//     match amount {
//         Some(amnt) => write!(f, "{} CCD", amnt.micro_ccd / 1000000),
//         None => write!(f, "0 CCD"),
//     }
// }

// pub fn debug_tokens_amount(amount: &TokenAmount, f: &mut concordium_std::fmt::Formatter) -> std::fmt::Result {
//     write!(f, "{} tokens", amount.0)
// }

// pub fn debug_timestamp(timestamp: &Timestamp, f: &mut concordium_std::fmt::Formatter) -> std::fmt::Result {
//     write!(f, "{} millis", timestamp.millis)
// }