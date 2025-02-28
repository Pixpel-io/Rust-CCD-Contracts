use concordium_cis2::{TokenAmountU64 as TokenAmount, TokenIdU8 as TokenID};
use concordium_std::{
    AccountAddress, Amount, ContractAddress, DeserialWithState, Duration, HasChainMetadata,
    HasCommonData, ReceiveContext, SchemaType, Serial, Serialize, StateApi, StateBuilder, StateMap,
    StateRef, StateRefMut, Timestamp,
};

use crate::{
    errors::LaunchPadError,
    params::{CreateParams, Months},
    ProductName,
};

/// Alias for mutable state reference of a `LaunchPad` type
pub type LaunchPadStateMut<'a> = StateRefMut<'a, LaunchPad, StateApi>;
/// Alias for immutable state reference of a `LaunchPad` type
pub type LaunchPadState<'a> = StateRef<'a, LaunchPad>;

/// Number of days in a month
pub const DAYS: u64 = 31;

/// The state of the smart contract.
/// This state can be viewed by querying the node with the command
/// `concordium-client contract invoke` using the `view` function as entry
/// point.
#[derive(Serial, DeserialWithState, Debug)]
#[concordium(state_parameter = "S")]
pub struct State<S = StateApi> {
    /// A mapping including all launchpad that have been added to this contract.
    pub launchpads: StateMap<ProductName, LaunchPad, S>,
    /// Container which holds the list of all the investors on the platform with
    /// associative list representing the launchpads in which they contribute
    pub investors: StateMap<AccountAddress, Vec<ProductName>, S>,
    /// Admin details of the contract
    pub admin: Admin,
    /// A counter that is sequentially increased whenever a new launchpad is added to
    /// the contract.
    pub counter: u32,
}

impl State {
    /// Getter function to get the platform registeration fee
    /// for launch-pad creation
    ///
    /// Returns `Amount` in CCD
    pub fn admin_registeration_fee(&self) -> Amount {
        self.admin.registeration_fee()
    }

    /// Gets the platform admin account address
    ///
    /// Resturns the `AccountAddress` type
    pub fn admin_address(&self) -> AccountAddress {
        self.admin.address
    }

    pub fn admin_allocation_share(&self) -> u64 {
        self.admin.allocation_share()
    }

    pub fn admin_liquidity_share(&self) -> u64 {
        self.admin.liquidity_share()
    }

    /// Gets the mutable reference to `LaunchPad` by product name with
    /// its associative ID
    ///
    /// Returns `LaunchPadError` if the LaunchPad does not exist.
    pub fn get_mut_launchpad(
        &mut self,
        product_name: String,
    ) -> Result<LaunchPadStateMut<'_>, LaunchPadError> {
        if let Some(launchpad) = self.launchpads.get_mut(&product_name) {
            return Ok(launchpad);
        }

        Err(LaunchPadError::NotFound)
    }

    /// Gets the immutable reference to `LaunchPad` by product name with
    /// its associative ID
    ///
    /// Returns `LaunchPadError` if the LaunchPad does not exist.
    pub fn get_launchpad(
        &self,
        product_name: String,
    ) -> Result<LaunchPadState<'_>, LaunchPadError> {
        if let Some(launchpad) = self.launchpads.get(&product_name) {
            return Ok(launchpad);
        }

        Err(LaunchPadError::NotFound)
    }

    pub fn my_launch_pads(
        &self,
        holder: AccountAddress,
    ) -> Result<Vec<ProductName>, LaunchPadError> {
        if let Some(ids) = self.investors.get(&holder) {
            return Ok(ids.clone());
        }

        Err(LaunchPadError::NotFound)
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
    pub holders: StateMap<AccountAddress, HolderInfo, S>,
    /// Defines the maximum and minimum investment amounts acceptable
    /// for presale
    pub vest_limits: VestingLimits,
    /// Details regarding the presale lock-up
    pub lock_up: Lockup,
    /// Details regarding the liquidity to lock and
    /// hold the funds
    pub liquidity_details: LiquidityDetails,
    /// Keeps track if the allocation share is paid
    pub allocation_paid: bool,
    /// Keeps track if the liquidity share is paid
    pub liquidity_paid: bool,
    /// Keeps track if the raised funds are already
    /// withdrawn
    pub withdrawn: bool,
}

impl LaunchPad {
    /// A constructor function to create a new `LaunchPad` instance
    /// from user parameters.
    ///
    /// Returns a `LaunchPad` and a 64-bit ID associated with it.
    pub fn from_create_params(
        params: CreateParams,
        state_builder: &mut StateBuilder,
    ) -> (ProductName, Self) {
        let cliff = params
            .launchpad_end_time()
            .checked_add(Duration::from_days(params.lockup_details.cliff * DAYS))
            .unwrap();
        (
            params.product.name.clone(),
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
                    cliff,
                    release_cycles: params.lockup_details.release_cycles,
                },
                liquidity_details: params.liquidity_details,
                allocation_paid: false,
                liquidity_paid: false,
                withdrawn: false,
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
        self.product.allocated_tokens
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

    /// Checks if the vesting is completed, by checking whether
    ///
    /// - Vesting duration has been elapsed
    ///
    /// Returns `true` if any of the above statement is true
    pub fn is_finished(&self, ctx: &ReceiveContext) -> bool {
        self.timeperiod.end > ctx.metadata().block_time()
    }

    /// Checks if the pause duration has elapsed
    pub fn is_pause_elapsed(&self, current: Timestamp) -> bool {
        self.pause.timeperiod.is_elapsed(current)
    }

    /// Checks if the Launch pad is caneled
    pub fn is_canceled(&self) -> bool {
        self.status == LaunchPadStatus::CANCELED
    }

    /// Checks if the Launch pad is caneled
    pub fn is_completed(&self) -> bool {
        self.status == LaunchPadStatus::COMPLETED
    }

    /// Returns the base price of allocated token for presale
    /// in CCD.
    pub fn product_base_price(&self) -> Amount {
        self.product.token_price
    }

    /// Returns the product name as `String`, for which the
    /// launch pad is created for presale.
    ///
    /// In other words, product name can be think of a single
    /// launch pad name for identification
    pub fn product_name(&self) -> ProductName {
        self.product.name.clone()
    }

    /// Returns the min token amount acceptable for vesting
    pub fn vest_min(&self) -> TokenAmount {
        self.vest_limits.min
    }

    /// Returns the max token amount acceptable for vesting
    pub fn vest_max(&self) -> TokenAmount {
        self.vest_limits.max
    }

    /// Checks if the soft cap is reached
    pub fn reached_soft_cap(&self) -> bool {
        self.collected >= self.soft_cap
    }

    /// Checks if the cliff duration has elapsed
    pub fn is_cliff_elapsed(&self, ctx: &ReceiveContext) -> bool {
        self.lock_up.cliff > ctx.metadata().block_time()
    }

    /// Gets the immutable reference to holder information
    /// releated to the launch pad.
    ///
    /// Returns `Ok()` if the holder exist or else returns `LaunchPadError`
    pub fn get_holder_info(
        &self,
        holder: AccountAddress,
    ) -> Result<StateRef<'_, HolderInfo>, LaunchPadError> {
        if let Some(info) = self.holders.get(&holder) {
            return Ok(info);
        }

        Err(LaunchPadError::WrongHolder)
    }

    /// Updates the release data related to a specific holder in the
    /// launch pad.
    pub fn set_holder_release_info(
        &mut self,
        holder: AccountAddress,
        cycle: u8,
        release_data: (TokenAmount, Timestamp),
    ) {
        let mut info = self.holders.get_mut(&holder).unwrap();

        info.cycles_rolled = cycle;
        let _ = info.release_data.insert(cycle, release_data);
    }
}

/// Defines the upper and lower bound limits for vesting.
/// Only the investments within these limits are accepted.
#[derive(Serialize, SchemaType, Clone, Debug)]
pub struct VestingLimits {
    /// Minimum amount in CCD acceptable for investment
    pub min: TokenAmount,
    /// Maximum amount in CCD acceptable for investment
    pub max: TokenAmount,
}

#[derive(Serialize, SchemaType, Clone, Debug)]
pub struct Product {
    /// Name if the product for which the Launchpad is created
    pub name: String,
    /// Address of the product owner
    pub owner: AccountAddress,
    /// Amount of tokens list for presale
    pub allocated_tokens: TokenAmount,
    /// Per token price decided by the owner for presale
    pub token_price: Amount,
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
    pub address: AccountAddress,
    /// Platform registeration fee to be paid by product
    pub registeration_fee: Amount,
    /// A certain percentage of shares to be paid by product
    /// once the soft-cap is reached in Tokens
    pub allocation_share: u64,
    /// A certain percentage from LP tokens will be charged
    /// by the platform
    pub liquidity_share: u64,
    /// Platform DEX contract, to handle and manage the LPTokens
    pub dex_address: ContractAddress
}

impl Admin {
    pub fn registeration_fee(&self) -> Amount {
        self.registeration_fee
    }

    pub fn allocation_share(&self) -> u64 {
        self.allocation_share
    }

    pub fn liquidity_share(&self) -> u64 {
        self.liquidity_share
    }
}

/// This type holds the information about a single holder and its
/// contributions in the launch pad, along with the details regarding
/// each release cycle related to the holder.
#[derive(Serial, DeserialWithState, Debug)]
#[concordium(state_parameter = "S")]
pub struct HolderInfo<S = StateApi> {
    /// Total amount of token bought by the holder
    pub tokens: TokenAmount,
    /// Total amount in CCD raised by the holder
    pub invested: Amount,
    /// How many release cycles have been claimed
    /// by the holder
    pub cycles_rolled: u8,
    /// Release data regarding each cycle claimed
    /// by the holder
    pub release_data: StateMap<u8, (TokenAmount, Timestamp), S>,
}

/// Holds the Lock-up details for launch-pad such as:
///
/// - Cliff durtaion
/// - Number of release cycles
///
/// Number of release cycles are actually the number of months chosen
/// by the product owner for linear release of allocated tokens
#[derive(Serialize, SchemaType, Debug, Clone)]
pub struct Lockup {
    /// Cliff duration of the launchpad based on
    /// number of months
    pub cliff: Timestamp,
    /// Number of cycles in which the vesting will be
    /// released, these cycles are based on number of months
    pub release_cycles: Months,
}

/// Holds the details regarding liquidity allocation of the raised funds
/// for launch-pad such as:
///
/// - Amount of funds to be locked in liquidity as LPTokens
/// - Number of release cycles
///
/// Number of release cycles are actually the number of months chosen
/// by the product owner for linear release of allocated tokens
#[derive(Serialize, SchemaType, Debug, Clone)]
pub struct LiquidityDetails {
    /// Amount of funds in percentage to be allocated and
    /// locked in liquidity within 40% to 60%
    pub liquidity_allocation: u64,
    /// Number of cycles in which the LPToken will be
    /// released, these cycles are based on number of months
    pub release_cycles: Months,
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
