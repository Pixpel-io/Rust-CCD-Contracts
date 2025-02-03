use concordium_cis2::TokenAmountU64;
use concordium_smart_contract_testing::{
    module_load_v1, Account, AccountKeys, Chain, Energy, InitContractPayload, Signer,
};
use concordium_std::{
    AccountAddress, AccountBalance, Address, Amount, ContractAddress, OwnedContractName,
    OwnedParameter, SignatureEd25519,
};
use concordium_std_derive::{account_address, signature_ed25519};

/// Alice dummy account for testing
pub const ALICE: AccountAddress =
    account_address!("2xBpaHottqhwFZURMZW4uZduQvpxNDSy46iXMYs9kceNGaPpZX");
pub const ALICE_ADDR: Address = Address::Account(ALICE);

/// Bob dummy account for testing
pub const BOB: AccountAddress =
    account_address!("2xdTv8awN1BjgYEw8W1BVXVtiEwG2b29U8KoZQqJrDuEqddseE");
pub const BOB_ADDR: Address = Address::Account(BOB);

/// Carol dummy account for testing
pub const CAROL: AccountAddress =
    account_address!("2y57FyMyqAfY7X1SuSWJ5VMt1Z3ZgxbKt9w5mGoTwqA7YcpbXr");

/// Dummy signer which always signs with one key
pub const SIGNER: Signer = Signer::with_one_key();

/// Account balance to initilize the test accounts
pub const ACC_INITIAL_BALANCE: Amount = Amount::from_ccd(10000);

/// Dummy signature
pub const DUMMY_SIGNATURE: SignatureEd25519 = signature_ed25519!("00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");

/// A helper function to setup and initialize the auction and cis2_multi contracts as mocks
/// for unit testing.
///
/// It is required to build the auction contract in the root as `build/auction.wasm.v1` and
/// cis2_multi build should be present in path `test-build-artifacts/cis2multi.wasm.v1`
pub fn initialize_chain_and_auction() -> (Chain, AccountKeys, ContractAddress, ContractAddress) {
    let mut chain = Chain::builder()
        .build()
        .expect("Should be able to build chain");

    // Create keys for ALICE.
    let rng = &mut rand::thread_rng();

    let keypairs_alice = AccountKeys::singleton(rng);

    let balance = AccountBalance {
        total: ACC_INITIAL_BALANCE,
        staked: Amount::zero(),
        locked: Amount::zero(),
    };

    // Create some accounts on the chain.
    chain.create_account(Account::new_with_keys(
        ALICE,
        balance,
        (&keypairs_alice).into(),
    ));
    chain.create_account(Account::new(BOB, ACC_INITIAL_BALANCE));
    chain.create_account(Account::new(CAROL, ACC_INITIAL_BALANCE));

    // Load and deploy the cis2 token module.
    let module = module_load_v1("./test-build-artifacts/cis2multi.wasm.v1").expect("Module exists");

    let deployment = chain
        .module_deploy_v1(SIGNER, CAROL, module)
        .expect("Deploy valid module");

    // Initialize the cis2 token contract.
    let token = chain
        .contract_init(
            SIGNER,
            CAROL,
            Energy::from(10000),
            InitContractPayload {
                amount: Amount::zero(),
                mod_ref: deployment.module_reference,
                init_name: OwnedContractName::new_unchecked("init_cis2_multi".to_string()),
                param: OwnedParameter::from_serial(&TokenAmountU64(100u64))
                    .expect("Serialize parameter"),
            },
        )
        .expect("Initialize cis2 token contract");

    // Load and deploy the auction module.
    let module = module_load_v1("build/auction.wasm.v1").expect("Module exists");
    let deployment = chain
        .module_deploy_v1(SIGNER, CAROL, module)
        .expect("Deploy valid module");

    // Initialize the auction contract.
    let init_auction = chain
        .contract_init(
            SIGNER,
            CAROL,
            Energy::from(10000),
            InitContractPayload {
                amount: Amount::zero(),
                mod_ref: deployment.module_reference,
                init_name: OwnedContractName::new_unchecked("init_cis2-auction".to_string()),
                param: OwnedParameter::empty(),
            },
        )
        .expect("Initialize auction");

    (
        chain,
        keypairs_alice,
        init_auction.contract_address,
        token.contract_address,
    )
}
