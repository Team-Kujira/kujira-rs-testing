#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    attr, coins, to_binary, BankMsg, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, Fraction,
    MessageInfo, Response, StdResult, Uint128,
};
use kujira::{
    msg::KujiraMsg,
    orca::{ExecuteMsg, InstantiateMsg, QueryMsg, SimulationResponse},
    query::KujiraQuery,
    utils::{amount, fee_address},
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
        ExecuteMsg::ExecuteLiquidation { exchange_rate, .. } => {
            let collateral_amount = amount(&COLLATERAL.into(), info.funds)?;

            let net_premium = Decimal::from_ratio(95u128, 100u128);
            let repay_amount = collateral_amount * exchange_rate * net_premium;
            let fee_amount = repay_amount * Decimal::from_ratio(1u128, 100u128);
            let repay_amount = repay_amount - fee_amount;

            Ok(Response::default()
                .add_attributes(vec![
                    attr("action", "execute_liquidation"),
                    attr("collateral_amount", collateral_amount),
                    attr("repay_amount", repay_amount),
                    attr("fee_amount", fee_amount),
                ])
                .add_messages(vec![
                    CosmosMsg::Bank(BankMsg::Send {
                        to_address: sender.to_string(),
                        amount: coins(repay_amount.u128(), STABLE.to_string()),
                    }),
                    CosmosMsg::Bank(BankMsg::Send {
                        to_address: fee_address().to_string(),
                        amount: coins(fee_amount.u128(), STABLE.to_string()),
                    }),
                ]))
        }
        _ => unimplemented!(),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps<KujiraQuery>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Simulate {
            collateral_amount,
            exchange_rate,
            ..
        } => {
            let net_premium = Decimal::from_ratio(95u128, 100u128);
            let repay_amount = collateral_amount * exchange_rate * net_premium;
            let fee_amount = repay_amount * Decimal::from_ratio(1u128, 100u128);
            let repay_amount = repay_amount - fee_amount;
            let res = SimulationResponse {
                collateral_amount,
                repay_amount,
            };

            to_binary(&res)
        }

        QueryMsg::SimulateReverse {
            repay_amount,
            exchange_rate,
            ..
        } => {
            // Add the 1% fee
            let fee_amount =
                repay_amount * Decimal::from_ratio(1u128, 99u128) 
                // Add 1 to compensate for decimal truncation Simulate. 
                // We want to ensure there's always enough collateral for the repay required
                + Uint128::from(1u128);

            let repay_amount = repay_amount + fee_amount;

            let collateral_value =
                repay_amount * Decimal::from_ratio(100u128, 95u128) + Uint128::from(1u128);

            let collateral_amount = Decimal::from_ratio(
                collateral_value * exchange_rate.denominator(),
                exchange_rate.numerator(),
            ) * Uint128::from(1u128)
                + Uint128::from(1u128);

            let res = SimulationResponse {
                collateral_amount,
                repay_amount,
            };

            to_binary(&res)
        }

        _ => unimplemented!(),
    }
}
