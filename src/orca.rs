#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    attr, coins, to_binary, BankMsg, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, Fraction,
    MessageInfo, Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw_storage_plus::Item;
use kujira::{
    msg::KujiraMsg,
    orca::{ExecuteMsg, InstantiateMsg, QueryMsg, SimulationResponse},
    query::KujiraQuery,
    utils::{amount, fee_address},
};

const STABLE: &str = "factory/contract0/uusk";
const COLLATERAL: &str = "factory/owner/coll";

const LIQUIDATION_FEE: Item<Decimal> = Item::new("liquidation_fee");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<KujiraQuery>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response<KujiraMsg>> {
    LIQUIDATION_FEE.save(deps.storage, &msg.liquidation_fee)?;
    Ok(Response::default())
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
        ExecuteMsg::ExecuteLiquidation { exchange_rate, callback, .. } => {
            let collateral_amount = amount(&COLLATERAL.into(), info.funds)?;

            let net_premium = Decimal::from_ratio(95u128, 100u128);
            let repay_amount = collateral_amount * exchange_rate * net_premium;
            let fee_amount = repay_amount * Decimal::from_ratio(1u128, 100u128);
            let repay_amount = repay_amount - fee_amount;

            let mut msgs = vec![];
            if fee_amount.gt(&Uint128::zero()) {
                msgs.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: fee_address().to_string(),
                    amount: coins(fee_amount.u128(), STABLE.to_string()),
                }));
            }

            match callback {
                None => msgs.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: sender.to_string(),
                    amount: coins(repay_amount.u128(), STABLE.to_string()),
                })),
                Some(cb) => msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: sender.to_string(),
                    funds: coins(fee_amount.u128(), STABLE.to_string()),
                    msg: cb,
                })),
            }

            Ok(Response::default()
                .add_attributes(vec![
                    attr("action", "execute_liquidation"),
                    attr("collateral_amount", collateral_amount),
                    attr("repay_amount", repay_amount),
                    attr("fee_amount", fee_amount),
                ])
                .add_messages(msgs))
        }
        _ => unimplemented!(),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<KujiraQuery>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Simulate {
            collateral_amount,
            exchange_rate,
            ..
        } => {
            let net_premium = Decimal::from_ratio(95u128, 100u128);
            let repay_amount = collateral_amount * exchange_rate * net_premium;
            let fee_amount = repay_amount * LIQUIDATION_FEE.load(deps.storage)?;
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
            let fee_amount = repay_amount * LIQUIDATION_FEE.load(deps.storage)?
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

        QueryMsg::SimulateWithTarget {
            collateral_amount,
            debt_amount,
            target_ltv,
            exchange_rate,
            ..
        } => {
            let mut remaining_collateral = collateral_amount;
            let mut remaining_debt = debt_amount;

            let premium_price = Decimal::from_ratio(95u128, 100u128) * exchange_rate;

            let num_term1 = {
                let decimals = target_ltv * exchange_rate;

                Decimal::from_ratio(
                    decimals.numerator() + Uint128::from(1u128),
                    decimals.denominator(),
                )
            } * remaining_collateral;
            let (numerator, numerator_negative) = if remaining_debt.gt(&num_term1) {
                (
                    // using num_term1 here makes a smaller numerator, so just use the normal floored math
                    remaining_debt - target_ltv * exchange_rate * remaining_collateral
                        + Uint128::from(1u128),
                    true,
                )
            } else {
                (num_term1 - remaining_debt + Uint128::from(1u128), false)
            };

            let (denominator, denominator_negative) =
                if premium_price.gt(&(target_ltv * exchange_rate)) {
                    // Want denominator to be smaller than required
                    let decimals = target_ltv * exchange_rate;
                    let decimals = Decimal::from_ratio(
                        decimals.numerator() + Uint128::from(1u128),
                        decimals.denominator(),
                    );
                    (premium_price - decimals, true)
                } else {
                    // But here, we want the first mul to be floored so as to make it smaller
                    (target_ltv * exchange_rate - premium_price, false)
                };
            if numerator_negative != denominator_negative {
                return Err(StdError::generic_err("Cannot liquidate to target LTV: numerator and denominator have different signs"));
            }
            let consumed_collateral = numerator * denominator.inv().unwrap() + Uint128::from(1u128);
            if consumed_collateral.gt(&remaining_collateral) {
                return Err(StdError::generic_err(format!(
                    "Cannot liquidate to target LTV: not enough collateral ({} > {})",
                    consumed_collateral, remaining_collateral
                )));
            }
            remaining_collateral -= consumed_collateral;
            remaining_debt -= consumed_collateral * premium_price;

            let repay_amount = debt_amount - remaining_debt;
            let fee_amount = repay_amount * LIQUIDATION_FEE.load(deps.storage)?;
            let repay_amount = repay_amount - fee_amount;
            let res = SimulationResponse {
                collateral_amount: collateral_amount - remaining_collateral,
                repay_amount,
            };
            Ok(to_binary(&res)?)
        }

        _ => unimplemented!(),
    }
}
