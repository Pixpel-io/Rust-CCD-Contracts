use concordium_cis2::{AdditionalData, Transfer};
use concordium_std::Amount;

use crate::{
    errors::Error,
    params::{BuyerParams, ListParams, Payment},
    state::{Price, TokenDetails, TokenIdentifier},
    tests::{buy_token, get_token_balance, transfer_tokens, BUYER, PIXP_TOKEN_ID, PIXP_TOKEN_URL, SELLER_TOKEN_ID_2, SELLER_TOKEN_URL_2},
};

use super::{
    initialize_chain_and_contracts, list_token, mint_token, update_operator_of, view_token_list,
    ADMIN, SELLER, SELLER_TOKEN_ID_1, SELLER_TOKEN_URL_1,
};

#[test]
fn market_place_smoke() -> Result<(), Error> {
    let (mut chain, _, market_palce, cis2_contract) = initialize_chain_and_contracts();

    mint_token(
        &mut chain,
        ADMIN,
        cis2_contract,
        PIXP_TOKEN_ID,
        PIXP_TOKEN_URL.to_string(),
    );

    transfer_tokens(
        &mut chain,
        ADMIN,
        Transfer {
            token_id: PIXP_TOKEN_ID,
            amount: 500.into(),
            from: ADMIN.into(),
            to: BUYER.into(),
            data: AdditionalData::empty(),
        },
        cis2_contract,
    );

    mint_token(
        &mut chain,
        SELLER,
        cis2_contract,
        SELLER_TOKEN_ID_1,
        SELLER_TOKEN_URL_1.to_string(),
    );

    mint_token(
        &mut chain,
        SELLER,
        cis2_contract,
        SELLER_TOKEN_ID_2,
        SELLER_TOKEN_URL_2.to_string(),
    );

    update_operator_of(&mut chain, ADMIN, market_palce.into(), cis2_contract)?;
    update_operator_of(&mut chain, BUYER, market_palce.into(), cis2_contract)?;
    update_operator_of(&mut chain, SELLER, market_palce.into(), cis2_contract)?;

    println!(
        "Admin CCD balance: {:?}\nSeller CCD balance: {:?}\nBuyer CCD balacne: {:?}",
        chain.account_balance(ADMIN),
        chain.account_balance(SELLER),
        chain.account_balance(BUYER),
    );

    list_token(
        &mut chain,
        SELLER,
        ListParams {
            id: SELLER_TOKEN_ID_1,
            cis2_address: cis2_contract,
            quantity: 1,
            price: Price::CCD(200),
        },
        market_palce,
    )?;

    list_token(
        &mut chain,
        SELLER,
        ListParams {
            id: SELLER_TOKEN_ID_2,
            cis2_address: cis2_contract,
            quantity: 1,
            price: Price::PIXP(50),
        },
        market_palce,
    )?;

    buy_token(
        &mut chain,
        BuyerParams {
            id: SELLER_TOKEN_ID_1,
            cis2_address: cis2_contract,
            quantity: 1,
            payment: Payment::CCD(200),
        },
        Amount::from_ccd(200),
        BUYER,
        market_palce,
    )?;

    buy_token(
        &mut chain,
        BuyerParams {
            id: SELLER_TOKEN_ID_2,
            cis2_address: cis2_contract,
            quantity: 1,
            payment: Payment::PIXP(50),
        },
        Amount::from_ccd(0),
        BUYER,
        market_palce,
    )?;

    // println!("{:#?}", view_token_list(&mut chain, ADMIN, market_palce));
    println!(
        "{:#?}",
        get_token_balance(
            &mut chain,
            ADMIN,
            &[
                (SELLER.into(), SELLER_TOKEN_ID_1),
                (BUYER.into(), SELLER_TOKEN_ID_1)
            ],
            cis2_contract
        )
    );

    println!(
        "{:#?}",
        get_token_balance(
            &mut chain,
            ADMIN,
            &[
                (SELLER.into(), SELLER_TOKEN_ID_2),
                (BUYER.into(), SELLER_TOKEN_ID_2)
            ],
            cis2_contract
        )
    );

    println!(
        "{:#?}",
        get_token_balance(
            &mut chain,
            ADMIN,
            &[
                (SELLER.into(), PIXP_TOKEN_ID),
                (BUYER.into(), PIXP_TOKEN_ID)
            ],
            cis2_contract
        )
    );
    Ok(())
}
