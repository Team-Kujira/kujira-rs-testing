use std::convert::TryFrom;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    coins, BankMsg, Binary, CosmosMsg, Decimal256, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Uint128, Uint256,
};
use kujira::{
    fin::{ExecuteMsg, InstantiateMsg, QueryMsg},
    msg::KujiraMsg,
    query::KujiraQuery,
    utils::amount,
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
            let amount: Uint256 = amount(&STABLE.to_string(), info.funds)?.into();
            let price = belief_price.unwrap_or_else(|| Decimal256::from_ratio(100u128, 1425u128));
            let return_amount: Uint128 = Uint128::try_from(amount * price)?;

            Ok(
                Response::default().add_messages(vec![CosmosMsg::Bank(BankMsg::Send {
                    to_address: sender.to_string(),
                    amount: coins(return_amount.u128(), COLLATERAL.to_string()),
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
