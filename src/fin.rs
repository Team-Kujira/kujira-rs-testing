use std::convert::TryFrom;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    coins, BankMsg, Binary, CosmosMsg, Decimal256, Deps, DepsMut, Env, Fraction, MessageInfo,
    Response, StdError, StdResult, Uint128, Uint256,
};
use kujira::{
    fin::{ExecuteMsg, InstantiateMsg, QueryMsg},
    msg::KujiraMsg,
    query::KujiraQuery,
};

const STABLE: &str = "factory/contract0/uusk";
const COLLATERAL: &str = "factory/owner/coll";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut<KujiraQuery>,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response<KujiraMsg>> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut<KujiraQuery>,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response<KujiraMsg>> {
    let sender = info.sender.clone();
    match msg {
        ExecuteMsg::Swap { belief_price, .. } => {
            let coin = info.funds[0].clone();
            let amount: Uint256 = coin.amount.into();
            let price = belief_price.unwrap_or_else(|| Decimal256::from_ratio(1425u128, 100u128));

            let (price, return_denom) = match coin.denom.as_str() {
                STABLE => (
                    Decimal256::from_ratio(price.denominator(), price.numerator()),
                    COLLATERAL,
                ),
                COLLATERAL => (price, STABLE),
                _ => return Err(StdError::generic_err("Invalid Denom")),
            };

            let return_amount: Uint128 = Uint128::try_from(amount * price)?;

            Ok(
                Response::default().add_messages(vec![CosmosMsg::Bank(BankMsg::Send {
                    to_address: sender.to_string(),
                    amount: coins(return_amount.u128(), return_denom),
                })]),
            )
        }
        _ => Ok(Response::default()),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps<KujiraQuery>, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    Ok(Binary::default())
}
