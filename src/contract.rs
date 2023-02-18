use astroport::pair::PoolResponse;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;
use cw20::MinterResponse;

use crate::error::ContractError;
use crate::msg::{InstantiateMsg, PriceResponse, QueryMsg};
use crate::state::{State, STATE};

use uints::U256;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:fair-lp-price-oracle";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// This module is purely a workaround that lets us ignore lints for all the code the `construct_uint!`
/// macro generates
#[allow(clippy::all)]
mod uints {
    uint::construct_uint! {
        pub struct U256(4);
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        owner: deps.api.addr_validate(&msg.owner)?,
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", msg.owner))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Price { lp_token_address } => to_binary(&query_price(deps, lp_token_address)?),
    }
}

fn query_price(deps: Deps, lp_token: Addr) -> StdResult<PriceResponse> {
    let minter_reponse: MinterResponse = deps
        .querier
        .query_wasm_smart(lp_token, &cw20::Cw20QueryMsg::Minter {})?;
    let pair_address = minter_reponse.minter;
    let pool_info: PoolResponse = deps
        .querier
        .query_wasm_smart(pair_address, &astroport::pair::QueryMsg::Pool {})?;

    // TODO: get price from oracle
    let base_price = 1;
    let quote_price = 1;

    // TODO: get actual asset precision
    let base_decimals = 6;
    let quote_decimals = 6;

    // RE the calculation of the value of liquidity token, see:
    // https://blog.alphafinance.io/fair-lp-token-pricing/
    // this formulation avoids a potential sandwich attack that distorts asset prices by a flashloan
    let base_value =
        U256::from(base_price * pool_info.assets[0].amount.u128() / 10u128.pow(base_decimals));
    let quote_value =
        U256::from(quote_price * pool_info.assets[1].amount.u128() / 10u128.pow(quote_decimals));
    let pool_value = U256::from(2) * (base_value * quote_value).integer_sqrt();
    let pool_value_uint128 = Uint128::new(pool_value.as_u128());
    let lp_token_price = Decimal::from_ratio(pool_value_uint128, pool_info.total_share);

    Ok(PriceResponse {
        rate: lp_token_price,
        // TODO: last_updated should be min(base_asset_price.last_updated, quote_asset_price.last_updated)
        last_updated: 0,
    })
}
