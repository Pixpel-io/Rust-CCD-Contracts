use concordium_cis2::TokenAmountU64;
use concordium_std::{collections::BTreeMap, *};
use twox_hash::xxh3::hash64;

use crate::{errors::LaunchPadError, params::CreateParams, types::*};

// #[derive(Serialize, Clone)]
// pub struct State {
//     pub total_launchpad: LaunchpadID, // length of launchpad
//     pub launchpad: BTreeMap<LaunchpadID, Launchpad>,
//     pub lockup_details: BTreeMap<LaunchpadID, LockupDetails>,
//     pub admin: AccountAddress,
// }

/// Launch-pad unique ID generated from product name
pub type LaunchPadID = u64;

/// AccountID is a base58 encoded string from raw bytes of account address
pub type AccountID = String;

/// Alias for `TokenAmountU64`
pub type TokenAmount = TokenAmountU64;

/// Alias which keeps track of release cycles
pub type CycleCount = u8;

/// The state of the smart contract.
/// This state can be viewed by querying the node with the command
/// `concordium-client contract invoke` using the `view` function as entry
/// point.
#[derive(Serial, DeserialWithState, Debug)]
#[concordium(state_parameter = "S")]
pub struct State<S = StateApi> {
    /// A mapping including all launchpad that have been added to this contract.
    pub launchpads: StateMap<LaunchPadID, LaunchPad, S>,
    /// Container which holds the list of all the investors on the platform with
    /// associative list representing the launchpads in which they contribute
    pub investors: StateMap<AccountAddress, LaunchPadID, S>,
    /// Admin details of the contract
    pub admin: Admin,
    /// A counter that is sequentially increased whenever a new launchpad is added to
    /// the contract.
    pub counter: u16,
}

impl State {
    /// Getter function to get the platform registeration fee
    /// for launch-pad creation
    /// 
    /// Returns `Amount` in CCD
    pub fn registeration_fee(&self) -> Amount {
        self.admin.registeration_fee
    }
}

#[derive(Serial, DeserialWithState, Debug)]
#[concordium(state_parameter = "S")]
pub struct LaunchPad<S = StateApi> {
    /// Product for which the presale is going to be established
    pub product: Product,
    /// Timeperiod of a launch-pad until it's expiry
    pub timeperiod: TimePeriod,
    /// Property which holds the status if the launchpad
    /// is `Live`, `paused`, `canceled` or `completed`
    pub status: LaunchPadStatus,
    /// Holds the details if the launchpad is paused
    pub pause: Option<PauseDetails>,
    /// Minimum limit of investment to reach before the
    /// launchpad expires
    pub soft_cap: Amount,
    /// Optional maximum limit of investment to reach before the
    /// launchpad expires
    pub hard_cap: Option<Amount>,
    /// Amount that have been collected sicne the start
    /// of presale
    pub collected: Amount,
    /// List of investors with their associated invested
    /// amount in CCD
    pub holders: StateMap<AccountAddress, Amount, S>,
    /// Defines the maximum and minimum investment amounts acceptable
    /// for presale
    pub vest_limits: VestingLimits,
    /// Details regarding the presale lock-up
    pub lock_up: Lockup,
}

impl LaunchPad {
    /// A constructor function to create a new `LaunchPad` instance
    /// from user parameters.
    ///
    /// Returns a `LaunchPad` and a 64-bit ID associated with it.
    pub fn from_create_params(
        params: CreateParams,
        state_builder: &mut StateBuilder,
    ) -> (LaunchPadID, Self) {
        (
            hash64(params.product.name.as_bytes()),
            Self {
                product: params.product,
                timeperiod: params.timeperiod,
                status: LaunchPadStatus::INREVIEW,
                pause: None,
                soft_cap: params.soft_cap,
                hard_cap: params.hard_cap,
                collected: Amount::zero(),
                holders: state_builder.new_map(),
                vest_limits: params.vest_limits,
                lock_up: Lockup {
                    cliff: params.lockup_details.cliff,
                    release_cycles: params.lockup_details.release_cycles,
                    cycles_rolled: 0,
                    cycle_details: Vec::new(),
                },
            },
        )
    }
}

/// Defines the duration interval of a Launch-pad during
/// which the Launch-pad remains active for presale
#[derive(Serialize, SchemaType, Clone, Debug)]
pub struct TimePeriod {
    /// Starting time of a launch-pad
    pub start: Timestamp,
    /// Ending time of a launch-pad
    pub end: Timestamp,
}

impl TimePeriod {
    /// Ensure whether the time period given is within the
    /// valid realistic range
    /// 
    /// Returns `Ok()` or else `VestingError`
    pub fn ensure_is_period_valid(&self, current: Timestamp) -> Result<(), LaunchPadError> {
        if self.start >= self.end && self.end <= current{
            return Err(LaunchPadError::InCorrectTimePeriod);
        }
        Ok(())
    }
}

/// Defines the upper and lower bound limits for vesting.
/// Only the investments within these limits are accepted.
#[derive(Serialize, SchemaType, Clone, Debug)]
pub struct VestingLimits {
    /// Minimum amount in CCD acceptable for investment
    pub min: Amount,
    /// Maximum amount in CCD acceptable for investment
    pub max: Amount,
}

#[derive(Serialize, SchemaType, Clone, Debug)]
pub struct Product {
    /// Name if the product for which the Launchpad is created
    pub name: String,
    /// Address of the product owner
    pub owner: AccountAddress,
    /// Amount of tokens list for presale
    pub token_amount: TokenAmount,
    /// Per token price decided by the owner for presale
    pub token_price: u32,
    /// Address of the CIS2 contract
    pub cis2_contract: ContractAddress,
}

#[derive(Serialize, SchemaType, Clone, Debug)]
pub enum LaunchPadStatus {
    /// When launchpas is approved and published for investments
    LIVE,
    /// When the launchpad is paused and not accepting investment
    PAUSED,
    /// When the launchpad is created and added in queue to be reviewed
    /// by an analyst before presale
    INREVIEW,
    /// When the launchpad is canceled by the owner or admin
    CANCELED,
    /// Once the launchpad has completed its cliff and vesting
    COMPLETED,
}

#[derive(Serialize, SchemaType, Clone, Debug)]
pub struct Admin {
    /// Admin account address to which all the fee
    /// must be transfered
    address: AccountAddress,
    /// Platform registeration fee to be paid by product
    registeration_fee: Amount,
    /// A certain percentage of shares to be paid by product
    /// once the soft-cap is reached
    token_allocation_cut: u8,
}

pub type ReleaseData = HashMap<AccountAddress, (TokenAmount, Timestamp)>;

#[derive(Serialize, SchemaType, Debug)]
pub struct Lockup {
    /// Cliff period of the launchpad
    pub cliff: Timestamp,
    /// Number of cycles in which the vesting will be released
    /// these cycles are based on number of months
    pub release_cycles: u8,
    /// Keeps track of cycles completed since the vesting started
    pub cycles_rolled: u8,
    /// Release details related to each cycle
    pub cycle_details: Vec<ReleaseData>,
}

#[derive(Serialize, SchemaType, Clone, Debug)]
pub struct PauseDetails {
    /// Pause period starting time
    pub start: Timestamp,
    /// Pause period ending time
    pub until: Timestamp,
    /// How many times the launchpas has been paused
    pub count: u8,
}
