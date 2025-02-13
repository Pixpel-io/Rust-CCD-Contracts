use concordium_cis2::{TokenAmountU64 as TokenAmount, TokenIdU8 as TokenID};
use concordium_std::{
    AccountAddress, Amount, ContractAddress, DeserialWithState, HashMap, SchemaType, Serial,
    Serialize, StateApi, StateBuilder, StateMap, StateRefMut, Timestamp,
};
use twox_hash::xxh3::hash64;

use crate::{errors::LaunchPadError, params::CreateParams};

/// Launch-pad unique ID generated from product name
pub type LaunchPadID = u64;

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
    pub investors: StateMap<AccountAddress, Vec<LaunchPadID>, S>,
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

    /// Gets the platform admin account address
    ///
    /// Resturns the `AccountAddress` type
    pub fn admin_address(&self) -> AccountAddress {
        self.admin.address
    }

    /// Gets the `LaunchPad` by product name.
    ///
    /// Returns `LaunchPadError` if the LaunchPad does not exist.
    pub fn get_launchpad(
        &mut self,
        product_name: String,
    ) -> Result<StateRefMut<'_, LaunchPad, StateApi>, LaunchPadError> {
        let launch_pad_id = hash64(product_name.as_bytes());

        if let Some(launchpad) = self.launchpads.get_mut(&launch_pad_id) {
            return Ok(launchpad);
        }

        Err(LaunchPadError::LaunchPadNotFound)
    }
}

#[derive(Serial, DeserialWithState, Debug)]
#[concordium(state_parameter = "S")]
pub struct LaunchPad<S = StateApi> {
    /// Product for which the presale is going to be established
    pub product: Product,
    /// Timeperiod of a launch-pad until it's expiry, in other words
    /// it defines the duration or vesting period
    pub timeperiod: TimePeriod,
    /// Property which holds the status if the launchpad
    /// is `Live`, `paused`, `canceled` or `completed`
    pub status: LaunchPadStatus,
    /// Holds the details if the launchpad is paused
    pub pause: PauseDetails,
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
                soft_cap: params.soft_cap,
                hard_cap: params.hard_cap,
                vest_limits: params.vest_limits,
                holders: state_builder.new_map(),
                status: LaunchPadStatus::INREVIEW,
                pause: PauseDetails::default(),
                collected: Amount::zero(),
                lock_up: Lockup {
                    cliff: params.lockup_details.cliff,
                    release_cycles: params.lockup_details.release_cycles,
                    cycles_rolled: 0,
                    cycle_details: Vec::new(),
                },
            },
        )
    }

    /// Getter method to get the CIS2 contract address related to
    /// a current launch-pad.
    ///
    /// Returns `ContractAddress`
    pub fn get_cis2_contract(&self) -> ContractAddress {
        self.product.cis2_contract
    }

    /// Getter method to get the amount of tokens listed for presale
    /// in current launch-pad.
    ///
    /// Returns `TokenAmount`
    pub fn get_product_token_amount(&self) -> TokenAmount {
        self.product.token_amount
    }

    /// Getter method to get the CIS2 token ID of tokens listed for
    /// presale in current launch-pad.
    ///
    /// Returns `TokenID`
    pub fn get_product_token_id(&self) -> TokenID {
        self.product.token_id
    }

    /// Getter method to get the owner account address of the  
    /// product in current launch-pad.
    ///
    /// Returns `AccountAddress`
    pub fn get_product_owner(&self) -> AccountAddress {
        self.product.owner
    }

    /// Get whether the launch-pad is live or not
    /// 
    /// Returns `ture` if live
    pub fn is_live(&self) -> bool {
        self.status == LaunchPadStatus::LIVE
    }

    /// Get whether the launch-pad is paused or not
    /// 
    /// Returns `ture` if paused
    pub fn is_paused(&self) -> bool {
        self.status == LaunchPadStatus::PAUSED
    }

    /// Get whether the launch-pad is live of Paused
    /// 
    /// Returns `ture` if live
    pub fn current_pause_count(&self) -> u8 {
        self.pause.count
    }

    /// Checks if the vesting time is finished
    pub fn is_vesting_finished(&self, current: Timestamp) -> bool {
        self.timeperiod.end < current
    }

    /// Checks if the pause duration has elapsed
    pub fn is_pause_elapsed(&self, current: Timestamp) -> bool {
        self.pause.timeperiod.is_elapsed(current)
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
    /// On chain token identifier in CIS2 contract
    pub token_id: TokenID,
}

#[derive(Serialize, SchemaType, Clone, Debug, PartialEq)]
pub enum LaunchPadStatus {
    /// When launchpas is approved and published for investments
    LIVE,
    /// When the launchpad is paused and not accepting investment
    PAUSED,
    /// When the launchpad is created and added in queue to be reviewed
    /// by an analyst before presale
    INREVIEW,
    /// When the Launch-pad is approved by analyst and now allowed to be
    /// listed for presale
    APPROVED,
    /// When the Launch-pad is rejected by analyst and not allowed to be
    /// listed for presale
    REJECTED,
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

/// Alias to hold details regarding vesting releases in each
/// release cycle rolled
pub type ReleaseData = HashMap<AccountAddress, (TokenAmount, Timestamp)>;

#[derive(Serialize, SchemaType, Debug)]
pub struct Lockup {
    /// Cliff period of the launchpad
    pub cliff: TimePeriod,
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
    /// Pause duration, should be greater than min
    /// pause duration 48 hrs
    pub timeperiod: TimePeriod,
    /// How many times the launchpas has been paused
    pub count: u8,
}

/// Default trait implementation for PauseDetails type
impl Default for PauseDetails {
    fn default() -> Self {
        Self {
            timeperiod: TimePeriod::default(),
            count: 0,
        }
    }
}

/// Defines the duration interval of a Launch-pad during
/// which the Launch-pad remains active for presale
#[derive(Serialize, SchemaType, Clone, Copy, Debug)]
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
        if self.start >= self.end && self.end <= current {
            return Err(LaunchPadError::InCorrectTimePeriod);
        }
        Ok(())
    }

    /// Gives the duration of a time period between start and end
    /// in milliseconds.
    ///
    /// Returns millis as `u64`
    pub fn duration_as_millis(&self) -> u64 {
        self.end.millis - self.start.millis
    }

    /// Returns the starting interval of a time period as `TimeStamp`
    pub fn start(&self) -> Timestamp {
        self.start
    }

    /// Returns the ending interval of a time period as `TimeStamp`
    pub fn end(&self) -> Timestamp {
        self.end
    }

    pub fn is_elapsed(&self, current: Timestamp) -> bool {
        self.end < current
    }
}

/// Default trait implementation for Timeperiod type
impl Default for TimePeriod {
    fn default() -> Self {
        Self {
            start: Timestamp::from(0),
            end: Timestamp::from(0),
        }
    }
}
