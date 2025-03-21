use crate::{
    add, calculate_amounts, list,
    params::AddParams,
    state::{Commission, State, TokenInfo, TokenListItem, TokenPriceState, TokenRoyaltyState},
    ContractState, ContractTokenAmount, ContractTokenId,
};
use concordium_cis2::*;

use concordium_std::{test_infrastructure::*, *};

const ACCOUNT_0: AccountAddress = AccountAddress([0u8; 32]);
const ADDRESS_0: Address = Address::Account(ACCOUNT_0);
const CIS_CONTRACT_ADDRESS: ContractAddress = ContractAddress {
    index: 1,
    subindex: 0,
};
const MARKET_CONTRACT_ADDRESS: ContractAddress = ContractAddress {
    index: 2,
    subindex: 0,
};

#[concordium_test]
fn should_add_token() {
    let token_id_1 = ContractTokenId::from(1);
    let token_quantity_1 = ContractTokenAmount::from(1);
    let price = Amount::from_ccd(1);

    let mut ctx = TestReceiveContext::default();
    ctx.set_sender(ADDRESS_0);
    ctx.set_self_address(MARKET_CONTRACT_ADDRESS);

    let add_params = AddParams {
        cis_contract_address: CIS_CONTRACT_ADDRESS,
        price,
        token_id: token_id_1,
        royalty: 0,
        quantity: token_quantity_1,
    };
    let parameter_bytes = to_bytes(&add_params);
    ctx.set_parameter(&parameter_bytes);

    let mut state_builder = TestStateBuilder::new();
    let state = State::new(&mut state_builder, 250);
    let mut host = TestHost::new(state, state_builder);

    fn mock_supports(
        _p: Parameter,
        _a: Amount,
        _a2: &mut Amount,
        _s: &mut ContractState<TestStateApi>,
    ) -> Result<(bool, SupportsQueryResponse), CallContractError<SupportsQueryResponse>>
    {
        Ok((
            false,
            SupportsQueryResponse {
                results: vec![SupportResult::Support],
            },
        ))
    }

    fn mock_is_operator_of(
        _p: Parameter,
        _a: Amount,
        _a2: &mut Amount,
        _s: &mut ContractState<TestStateApi>,
    ) -> Result<(bool, OperatorOfQueryResponse), CallContractError<OperatorOfQueryResponse>>
    {
        Ok((false, OperatorOfQueryResponse { 0: vec![true] }))
    }

    fn mock_balance_of(
        _p: Parameter,
        _a: Amount,
        _a2: &mut Amount,
        _s: &mut ContractState<TestStateApi>,
    ) -> Result<
        (bool, BalanceOfQueryResponse<ContractTokenAmount>),
        CallContractError<BalanceOfQueryResponse<ContractTokenAmount>>,
    > {
        Ok((false, BalanceOfQueryResponse(vec![1.into()])))
    }

    TestHost::setup_mock_entrypoint(
        &mut host,
        CIS_CONTRACT_ADDRESS,
        OwnedEntrypointName::new_unchecked("supports".to_string()),
        MockFn::new_v1(mock_supports),
    );

    TestHost::setup_mock_entrypoint(
        &mut host,
        CIS_CONTRACT_ADDRESS,
        OwnedEntrypointName::new_unchecked("operatorOf".to_string()),
        MockFn::new_v1(mock_is_operator_of),
    );

    TestHost::setup_mock_entrypoint(
        &mut host,
        CIS_CONTRACT_ADDRESS,
        OwnedEntrypointName::new_unchecked("balanceOf".to_string()),
        MockFn::new_v1(mock_balance_of),
    );

    let res = add(&ctx, &mut host);

    claim!(res.is_ok(), "Results in rejection");
    claim!(
        host.state().token_prices.iter().count() != 0,
        "Token not added"
    );
    claim!(
        host.state().token_royalties.iter().count() != 0,
        "Token not added"
    );
    claim_eq!(
        host.state().commission,
        Commission {
            percentage_basis: 250,
        }
    );

    let token_list_tuple = host
        .state()
        .get_listed(
            &TokenInfo {
                id: token_id_1,
                address: CIS_CONTRACT_ADDRESS,
            },
            &ACCOUNT_0,
        )
        .expect("Should not be None");

    claim_eq!(
        token_list_tuple.0.to_owned(),
        TokenRoyaltyState {
            primary_owner: ACCOUNT_0,
            royalty: 0,
        }
    );
    claim_eq!(
        token_list_tuple.1.to_owned(),
        TokenPriceState {
            price,
            quantity: token_quantity_1
        },
    )
}

#[concordium_test]
fn should_list_token() {
    let token_quantity_1 = ContractTokenAmount::from(1);
    let token_id_1 = ContractTokenId::from(1);
    let token_id_2 = ContractTokenId::from(2);
    let token_price_1 = Amount::from_ccd(1);
    let token_price_2 = Amount::from_ccd(2);

    let mut ctx = TestReceiveContext::default();
    ctx.set_sender(ADDRESS_0);
    ctx.set_self_address(MARKET_CONTRACT_ADDRESS);

    let mut state_builder = TestStateBuilder::new();
    let mut state = State::new(&mut state_builder, 250);
    state.list_token(
        &TokenInfo {
            id: token_id_1,
            address: CIS_CONTRACT_ADDRESS,
        },
        &ACCOUNT_0,
        token_price_1,
        0,
        token_quantity_1,
    );
    state.list_token(
        &TokenInfo {
            id: token_id_2,
            address: CIS_CONTRACT_ADDRESS,
        },
        &ACCOUNT_0,
        token_price_2,
        0,
        token_quantity_1,
    );
    let host = TestHost::new(state, state_builder);
    let list_result = list(&ctx, &host);

    claim!(list_result.is_ok());
    let token_list = list_result.unwrap();
    let list = token_list.0;
    claim_eq!(list.len(), 2);

    let first_token = list.first().unwrap();
    let second_token = list.last().unwrap();

    claim_eq!(
        first_token,
        &TokenListItem {
            token_id: token_id_1,
            contract: CIS_CONTRACT_ADDRESS,
            price: token_price_1,
            owner: ACCOUNT_0,
            primary_owner: ACCOUNT_0,
            quantity: token_quantity_1,
            royalty: 0,
        }
    );

    claim_eq!(
        second_token,
        &TokenListItem {
            token_id: token_id_2,
            contract: CIS_CONTRACT_ADDRESS,
            price: token_price_2,
            owner: ACCOUNT_0,
            primary_owner: ACCOUNT_0,
            quantity: token_quantity_1,
            royalty: 0,
        }
    )
}

#[concordium_test]
fn calculate_commissions_test() {
    let commission_percentage_basis: u16 = 250;
    let royalty_percentage_basis: u16 = 1000;
    let init_amount = Amount::from_ccd(11);
    let distributable_amounts = calculate_amounts(
        &init_amount,
        &Commission {
            percentage_basis: commission_percentage_basis,
        },
        royalty_percentage_basis,
    );

    claim_eq!(
        distributable_amounts.to_seller,
        Amount::from_micro_ccd(9625000)
    );
    claim_eq!(
        distributable_amounts.to_marketplace,
        Amount::from_micro_ccd(275000)
    );
    claim_eq!(
        distributable_amounts.to_primary_owner,
        Amount::from_micro_ccd(1100000)
    );
    claim_eq!(
        init_amount,
        Amount::from_ccd(0)
            .add_micro_ccd(distributable_amounts.to_seller.micro_ccd())
            .add_micro_ccd(distributable_amounts.to_marketplace.micro_ccd())
            .add_micro_ccd(distributable_amounts.to_primary_owner.micro_ccd())
    )
}