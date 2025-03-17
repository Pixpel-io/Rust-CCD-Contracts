use crate::{
    errors::LaunchPadError,
    params::{ApprovalParams, CreateParams, VestParams},
    response::{LaunchPadView, StateView},
    state::Admin,
};
use concordium_cis2::{
    AdditionalData, BalanceOfQuery, BalanceOfQueryParams, BalanceOfQueryResponse, OperatorOfQuery,
    OperatorOfQueryParams, OperatorOfQueryResponse, OperatorUpdate, Receiver,
    TokenAmountU64 as TokenAmount, TokenIdU8 as TokenID, Transfer, TransferParams, UpdateOperator,
    UpdateOperatorParams,
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

// mod bid;
// mod cancel;
// mod finalize;
// mod item;
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

/// A helper function to setup and initialize the auction and cis2_multi contracts as mocks
/// for unit testing.
///
/// It is required to build the auction contract in the root as `build/auction.wasm.v1` and
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
            dex_address: ContractAddress::new(1001, 0),
        },
    );

    // Load and deploy the DEX (Pixpel swap) module.
    let dex_contract = initialize_contract(
        &mut chain,
        "../nft-auction/test-build-artifacts/pixpel_swap.wasm.v1".into(),
        "pixpel_swap".into(),
        (),
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
        Energy::from(10000),
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
    chain: &Chain,
    invoker: AccountAddress,
    balance_of: Address,
    cis2_contract: ContractAddress,
    token_id: TokenID,
) -> BalanceOfQueryResponse<TokenAmount> {
    let balance_of_params: BalanceOfQueryParams<_> = BalanceOfQueryParams {
        queries: vec![BalanceOfQuery {
            token_id,
            address: balance_of,
        }],
    };

    let payload = UpdateContractPayload {
        amount: Amount::zero(),
        receive_name: OwnedReceiveName::new_unchecked("cis2_multi.balanceOf".to_string()),
        address: cis2_contract,
        message: OwnedParameter::from_serial(&balance_of_params).expect("BalanceOf params"),
    };

    let invoke = chain
        .contract_invoke(
            invoker,
            Address::Account(invoker),
            Energy::from(10000),
            payload,
        )
        .expect("[Error] Balance_Of query Invocation failed");

    invoke
        .parse_return_value()
        .expect("[Error] Unable to deserialize response Balance_Of quary")
}

/// A helper function which invokes `cis2_multi` contract to update the operator of a certain
/// account or contract.
///
/// This is useful for integration testing
fn update_operator_of(
    chain: &mut Chain,
    invoker: AccountAddress,
    sender: Address,
    operator_to_be: Address,
    cis2_contract: ContractAddress,
) {
    let update_operator_params = UpdateOperatorParams(vec![UpdateOperator {
        update: OperatorUpdate::Add,
        operator: operator_to_be,
    }]);

    let () = update_contract(
        chain,
        cis2_contract,
        invoker,
        update_operator_params,
        None,
        "cis2_multi.updateOperator",
    )
    .expect("[Error] While invoking update_operator");
}

/// A helper function which invokes `cis2_multi` to check if an account or contract is
/// operator of an owner in ci2_contract
///
/// This is useful for integration testing
fn ensure_is_operator_of(
    chain: &mut Chain,
    invoker: AccountAddress,
    sender: Address,
    is_operator: Address,
    cis2_contract: ContractAddress,
) -> bool {
    let is_operator_params = OperatorOfQueryParams {
        queries: vec![OperatorOfQuery {
            owner: sender,
            address: is_operator,
        }],
    };

    let response: OperatorOfQueryResponse = update_contract(
        chain,
        cis2_contract,
        invoker,
        is_operator_params,
        None,
        "cis2_multi.operatorOf",
    )
    .expect("[Error] While invoking ensure_is_operator");

    response.0[0]
}

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
/// A helper function which invokes `cis2_multi` to check if an account or contract is
/// operator of an owner in ci2_contract
///
/// This is useful for integration testing
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

/// A helper function to invoke `viewItemState` in auction to get a specefic
/// item's current state in the auction contract
///
/// Returns the `ItemState` type or panics with error message
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

fn view_state(chain: &mut Chain, invoker: AccountAddress, contract: ContractAddress) -> StateView {
    read_contract(chain, contract, invoker, (), "LaunchPad.viewState")
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
