use crate::{
    errors::LaunchPadError,
    params::{ApprovalParams, ClaimLockedParams, ClaimUnLockedParams, CreateParams, VestParams},
    response::LaunchPadView,
    state::Admin,
};
use concordium_cis2::{
    AdditionalData, BalanceOfQuery, BalanceOfQueryParams, BalanceOfQueryResponse, Receiver,
    TokenAmountU64 as TokenAmount, TokenIdU64, TokenIdU8 as TokenID, Transfer, TransferParams
};
use concordium_smart_contract_testing::{
    module_load_v1, Account, AccountKeys, Chain, Energy, InitContractPayload, Signer,
    UpdateContractPayload,
};
use concordium_std::{
    AccountAddress, AccountBalance, Address, Amount, ContractAddress, Deserial, MetadataUrl,
    OwnedContractName, OwnedEntrypointName, OwnedParameter, OwnedReceiveName, SchemaType, Serial,
    ACCOUNT_ADDRESS_SIZE,
};

mod smoke;

/// Dummy signer which always signs with one key
const SIGNER: Signer = Signer::with_one_key();

/// Account balance to initilize the test accounts
const ACC_INITIAL_BALANCE: Amount = Amount::from_ccd(20000);

const ADMIN: AccountAddress = AccountAddress([1; ACCOUNT_ADDRESS_SIZE]);
const OWNER: AccountAddress = AccountAddress([2; ACCOUNT_ADDRESS_SIZE]);

const HOLDERS: &'static [AccountAddress] = &[
    AccountAddress([3; ACCOUNT_ADDRESS_SIZE]),
    AccountAddress([4; ACCOUNT_ADDRESS_SIZE]),
    AccountAddress([5; ACCOUNT_ADDRESS_SIZE]),
];

const PLATFORM_REG_FEE: Amount = Amount::from_ccd(10);
const LIQUID_SHARE: u64 = 2;
const ALLOC_SHARE: u64 = 1;

const OWNER_TOKEN_ID: TokenID = TokenID(1);
const OWNER_TOKEN_URL: &str = "http://some.example/token/0";

/// A helper function to setup and initialize the concordium block-chain and deploy the contracts as mocks
/// for unit testing.
///
/// It is required to build the `Dex`, `LaunchPad`, `Cis2_multi` contracts in the root as `build/auction.wasm.v1` and
/// cis2_multi build should be present in path `test-build-artifacts/cis2multi.wasm.v1`
pub fn initialize_chain_and_contracts() -> (
    Chain,
    AccountKeys,
    ContractAddress,
    ContractAddress,
    ContractAddress,
) {
    let mut chain = Chain::builder()
        .build()
        .expect("Should be able to build chain");

    // Create keys for ALICE.
    let rng = &mut rand::thread_rng();

    let keypairs_admin = AccountKeys::singleton(rng);

    let balance = AccountBalance {
        total: ACC_INITIAL_BALANCE,
        staked: Amount::zero(),
        locked: Amount::zero(),
    };

    // Create some accounts on the chain.
    chain.create_account(Account::new_with_keys(
        ADMIN,
        balance,
        (&keypairs_admin).into(),
    ));

    for acc_addr in [OWNER].iter().chain(HOLDERS.iter()) {
        chain.create_account(Account::new(*acc_addr, ACC_INITIAL_BALANCE));
    }

    // Load and deploy the cis2 token module.
    let cis2_contract = initialize_contract(
        &mut chain,
        "../nft-auction/test-build-artifacts/cis2multi.wasm.v1",
        "cis2_multi",
        TokenAmount(10000u64),
    );

    // Load and deploy the DEX (Pixpel swap) module.
    let dex_contract = initialize_contract(
        &mut chain,
        "../nft-auction/test-build-artifacts/pixpel_swap.wasm.v1".into(),
        "pixpel_swap".into(),
        (),
    );

    // Load and deploy the main Launch Pad module.
    let launch_pad_contract = initialize_contract(
        &mut chain,
        "build/launchpad.wasm.v1",
        "LaunchPad",
        Admin {
            address: ADMIN,
            registeration_fee: PLATFORM_REG_FEE,
            liquidity_share: LIQUID_SHARE,
            allocation_share: ALLOC_SHARE,
            dex_address: dex_contract,
        },
    );

    (
        chain,
        keypairs_admin,
        launch_pad_contract,
        cis2_contract,
        dex_contract,
    )
}

fn initialize_contract<P>(
    chain: &mut Chain,
    module_path: &str,
    contract_name: &str,
    init_params: P,
) -> ContractAddress
where
    P: Serial,
{
    let module = module_load_v1(module_path).expect("[Error] Unable to load module");
    let deploy = chain
        .module_deploy_v1(SIGNER, ADMIN, module)
        .expect("[Error] Unable to deploy");

    let owned_params = OwnedParameter::from_serial(&init_params).unwrap();

    let payload = InitContractPayload {
        amount: Amount::zero(),
        mod_ref: deploy.module_reference,
        init_name: OwnedContractName::new_unchecked(format!("init_{}", contract_name)),
        param: owned_params,
    };

    chain
        .contract_init(SIGNER, ADMIN, Energy::from(10000), payload)
        .expect("[Error] Unable to initialize contract")
        .contract_address
}

fn update_contract<P, R>(
    chain: &mut Chain,
    contract: ContractAddress,
    invoker: AccountAddress,
    params: P,
    payable: Option<Amount>,
    receive_name: &str,
) -> Result<R, LaunchPadError>
where
    P: Serial,
    R: Deserial,
{
    let amount = match payable {
        Some(amount) => amount,
        None => Amount::zero(),
    };

    let payload = UpdateContractPayload {
        amount,
        address: contract,
        receive_name: OwnedReceiveName::new_unchecked(receive_name.to_string()),
        message: OwnedParameter::from_serial(&params).unwrap(),
    };

    let result = chain.contract_update(
        SIGNER,
        invoker,
        Address::Account(invoker),
        Energy::from(20000),
        payload,
    );

    match result {
        Ok(success) => match success.parse_return_value() {
            Ok(ret_type) => Ok(ret_type),
            Err(pe) => Err(pe.into()),
        },
        Err(ce) => Err(ce.into()),
    }
}

fn read_contract<P, R>(
    chain: &mut Chain,
    contract: ContractAddress,
    invoker: AccountAddress,
    params: P,
    receive_name: &str,
) -> R
where
    P: Serial,
    R: Deserial,
{
    let payload = UpdateContractPayload {
        amount: Amount::zero(),
        address: contract,
        receive_name: OwnedReceiveName::new_unchecked(receive_name.to_string()),
        message: OwnedParameter::from_serial(&params).expect("[Error] Unable to parse params"),
    };

    let result = chain.contract_invoke(
        invoker,
        Address::Account(invoker),
        Energy::from(10000),
        payload,
    );

    result
        .unwrap()
        .parse_return_value()
        .expect("[Error] Unable to deserialize response")
}

/// The parameter for the contract function `mint` which mints/airdrops a number
/// of tokens to the owner's address.
#[derive(Serial, SchemaType, Clone)]
pub struct MintParams {
    /// Owner of the newly minted tokens.
    pub to: Receiver,
    /// The metadata_url of the token.
    pub metadata_url: MetadataUrl,
    /// The token_id to mint/create additional tokens.
    pub token_id: TokenID,
    /// Additional data that can be sent to the receiving contract.
    pub data: AdditionalData,
}

impl From<(AccountAddress, TokenID, String)> for MintParams {
    fn from(value: (AccountAddress, TokenID, String)) -> Self {
        Self {
            to: Receiver::from_account(value.0),
            metadata_url: MetadataUrl {
                url: value.2,
                hash: None,
            },
            token_id: value.1,
            data: AdditionalData::empty(),
        }
    }
}

/// A helper function which invokes `cis2_multi` contract to `mint` airdrop tokens for the given
/// account.
///
/// This is useful for minting some mock tokens to be tested in integration tests by auction
/// contract
pub fn mint_token(
    chain: &mut Chain,
    account: AccountAddress,
    cis2_contract: ContractAddress,
    token_id: TokenID,
    url: String,
) {
    let params = MintParams::from((account, token_id, url));

    update_contract::<_, ()>(
        chain,
        cis2_contract,
        account,
        params,
        None,
        "cis2_multi.mint",
    )
    .expect("[Error] Mint Failed");
}

/// A helper function which invokes `cis2_multi` contract to get the balance of specific tokens minted
/// for a specifi account.
///
/// This is useful for integration testing
pub fn get_token_balance(
    chain: &mut Chain,
    invoker: AccountAddress,
    balance_of: &[(Address, TokenID)],
    cis2_contract: ContractAddress,
) -> BalanceOfQueryResponse<TokenAmount> {
    let queries: Vec<_> = balance_of
        .iter()
        .map(|(address, token_id)| BalanceOfQuery {
            token_id: *token_id,
            address: *address,
        })
        .collect::<Vec<_>>();

    read_contract(
        chain,
        cis2_contract,
        invoker,
        BalanceOfQueryParams { queries },
        "cis2_multi.balanceOf",
    )
}

/// A helper function which invokes `Dex` contract to get the balance of specific LPTokens`
/// for a specifi account.
pub fn get_lp_token_balance(
    chain: &mut Chain,
    invoker: AccountAddress,
    balance_of: &[(Address, TokenIdU64)],
    dex_contract: ContractAddress,
) -> BalanceOfQueryResponse<TokenAmount> {
    let queries: Vec<_> = balance_of
        .iter()
        .map(|(address, token_id)| BalanceOfQuery {
            token_id: *token_id,
            address: *address,
        })
        .collect::<Vec<_>>();

    read_contract(
        chain,
        dex_contract,
        invoker,
        BalanceOfQueryParams { queries },
        "pixpel_swap.balanceOf",
    )
}

/// A helper function which invokes `ClaimLockedTokens` method in launch pad. This
/// method is invoked by the either the owner or holder to claim their locked funds
/// in liquidity pool as LPTokens.
fn claim_locked_tokens(
    chain: &mut Chain,
    invoker: AccountAddress,
    params: ClaimLockedParams,
    contract: ContractAddress,
) -> Result<(), LaunchPadError> {
    update_contract(
        chain,
        contract,
        invoker,
        params,
        None,
        "LaunchPad.WithDrawLockedFunds",
    )
}

/// A helper function which invokes `ClaimTokens` method in launch pad. This
/// method is invoked by the holder to claim his tokens bought in launch pad.
fn claim_tokens(
    chain: &mut Chain,
    invoker: AccountAddress,
    params: ClaimUnLockedParams,
    contract: ContractAddress,
) -> Result<(), LaunchPadError> {
    update_contract(
        chain,
        contract,
        invoker,
        params,
        None,
        "LaunchPad.ClaimTokens",
    )
}

/// A helper function which invokes `WithdrawFunds` method in launch pad. This
/// method is invoked by the product owner to retrieve the raised amount into
/// wallet.
fn withdraw_raised_funds(
    chain: &mut Chain,
    invoker: AccountAddress,
    params: String,
    contract: ContractAddress,
) -> Result<(), LaunchPadError> {
    update_contract(
        chain,
        contract,
        invoker,
        params,
        None,
        "LaunchPad.WithdrawFunds",
    )
}

/// A helper function which invokes `Vest` method in launch pad contract.
fn invest(
    chain: &mut Chain,
    invoker: AccountAddress,
    params: VestParams,
    amount: Amount,
    contract: ContractAddress,
) -> Result<(), LaunchPadError> {
    update_contract(
        chain,
        contract,
        invoker,
        params,
        Some(amount),
        "LaunchPad.Vest",
    )
}
/// A helper function which invokes `cis2 transfer`, which in turns invokes the
/// "Depsoit" method in launch pad.
fn deposit_tokens(
    chain: &mut Chain,
    invoker: AccountAddress,
    product_name: String,
    cis2_contract: ContractAddress,
    launch_pad_contract: ContractAddress,
) -> Result<(), LaunchPadError> {
    let transfer_params = TransferParams(vec![Transfer {
        token_id: OWNER_TOKEN_ID,
        amount: TokenAmount(10000),
        from: Address::Account(OWNER),
        to: Receiver::Contract(
            launch_pad_contract,
            OwnedEntrypointName::new_unchecked("Deposit".to_string()),
        ),
        data: AdditionalData::from(product_name.as_bytes().to_owned()),
    }]);

    update_contract::<_, ()>(
        chain,
        cis2_contract,
        invoker,
        transfer_params,
        None,
        "cis2_multi.transfer",
    )
}

/// A helper function to invoke `ApproveLaunchPad` in contract to approve/reject
/// the launch pad
fn approve_launch_pad(
    chain: &mut Chain,
    invoker: AccountAddress,
    params: ApprovalParams,
    contract: ContractAddress,
) -> Result<(), LaunchPadError> {
    update_contract(
        chain,
        contract,
        invoker,
        params,
        None,
        "LaunchPad.ApproveLaunchPad",
    )
}

/// A helper function to invoke `viewLauchPad` in launch pad to get a specefic
/// launch pad current state in the contract
///
/// Returns the `LaunchPadView` type or panics with error message
fn view_launch_pad(
    chain: &mut Chain,
    invoker: AccountAddress,
    product_name: String,
    contract: ContractAddress,
) -> LaunchPadView {
    read_contract(
        chain,
        contract,
        invoker,
        product_name,
        "LaunchPad.viewLaunchPad",
    )
}

/// A helper function to invoke `CreatLaunchPad` function in launch pad contract to list an
/// product for ICO presale
///
/// Returns the `Ok()` if the invocation succeeds or else `LaunchPadError::Error`
fn create_launch_pad(
    chain: &mut Chain,
    contract: ContractAddress,
    invoker: AccountAddress,
    add_params: CreateParams,
) -> Result<(), LaunchPadError> {
    update_contract(
        chain,
        contract,
        invoker,
        add_params,
        Some(PLATFORM_REG_FEE),
        "LaunchPad.CreateLaunchPad",
    )
}
