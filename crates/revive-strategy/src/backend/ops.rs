use std::u64;

use alloy_primitives::{Bytes, FixedBytes, Log, LogData};
use foundry_evm::backend::DatabaseExt;
use itertools::Itertools;
use revive_env::{AccountId, Runtime};

use polkadot_sdk::{
    pallet_revive::{evm::GasEncoder, AddressMapper, BalanceOf, Config, DepositLimit, Pallet},
    polkadot_sdk_frame::prelude::OriginFor,
    sp_core::{self, H160},
    sp_weights::Weight,
};

use revm::{
    interpreter::{CallInputs, Gas},
    primitives::{ExecutionResult, ResultAndState},
};

use crate::{backend::get_backend_mut, trace, tracing::apply_prestate_trace};

pub fn try_call(
    backend: &mut foundry_evm::backend::Backend,
    env: &mut revm::primitives::EnvWithHandlerCfg,
) -> eyre::Result<ResultAndState> {
    let call = CallInputs::new(&env.tx, env.block.gas_limit.to()).expect("should be a call");

    let ctx = get_backend_mut(backend.get_strategy().context.as_mut()).clone();
    let mut ecx = revm::InnerEvmContext::new_with_env(backend.db_mut(), env.env.clone());

    tracing::info!("running call in PVM {:#?}", call);

    let max_gas = <<Runtime as Config>::EthGasEncoder as GasEncoder<BalanceOf<Runtime>>>::encode(
        Default::default(),
        Weight::MAX,
        1u128 << 99,
    );
    let gas_limit = sp_core::U256::from(call.gas_limit).min(max_gas);

    let (res, call_trace, prestate_trace) =
        ctx.revive_test_externalities.lock().unwrap().execute_with(|| {
            trace::<Runtime, _, _>(|| {
                let origin = OriginFor::<Runtime>::signed(AccountId::to_fallback_account_id(
                    &H160::from_slice(call.caller.as_slice()),
                ));
                let evm_value = sp_core::U256::from_little_endian(&call.call_value().as_le_bytes());

                let (gas_limit, storage_deposit_limit) =
                    <<Runtime as Config>::EthGasEncoder as GasEncoder<BalanceOf<Runtime>>>::decode(
                        gas_limit,
                    )
                    .expect("gas limit is valid");
                let storage_deposit_limit = DepositLimit::Balance(storage_deposit_limit);
                let target = H160::from_slice(call.target_address.as_slice());

                Pallet::<Runtime>::bare_call(
                    origin,
                    target,
                    evm_value,
                    gas_limit,
                    storage_deposit_limit,
                    call.input.to_vec(),
                )
            })
        });
    let mut gas = Gas::new(call.gas_limit);
    let gas_used = <<Runtime as Config>::EthGasEncoder as GasEncoder<BalanceOf<Runtime>>>::encode(
        gas_limit,
        res.gas_required,
        res.storage_deposit.charge_or_zero(),
    );

    apply_prestate_trace(prestate_trace, &mut ecx);
    let execution_result = {
        match res.result {
            Ok(result) => {
                let _ = gas.record_cost(gas_used.as_u64());
                let outcome = if result.did_revert() {
                    ExecutionResult::Revert {
                        gas_used: gas_used.try_into().unwrap_or(u64::MAX),
                        output: call_trace.map(|x| x.output.0.into()).unwrap_or(result.data.into()),
                    }
                } else {
                    ExecutionResult::Success {
                        reason: revm::primitives::SuccessReason::Return,
                        gas_used: gas.spent_sub_refunded(),
                        gas_refunded: 0,
                        logs: call_trace
                            .map(|x| {
                                x.logs
                                    .iter()
                                    .map(|x| Log {
                                        address: alloy_primitives::Address::from(x.address.0),
                                        data: LogData::new_unchecked(
                                            x.topics
                                                .iter()
                                                .map(|topic| FixedBytes::<32>(topic.0))
                                                .collect_vec(),
                                            x.data.clone().0.into(),
                                        ),
                                    })
                                    .collect_vec()
                            })
                            .unwrap_or(Default::default()),
                        output: revm::primitives::Output::Call(result.data.into()),
                    }
                };

                outcome
            }
            Err(e) => {
                tracing::error!("Contract call failed: {e:#?}");
                ExecutionResult::Revert {
                    gas_used: gas_used.try_into().unwrap_or(u64::MAX),
                    output: Bytes::from_iter(format!("Contract call failed: {e:#?}").as_bytes()),
                }
            }
        }
    };
    let result =
        ResultAndState { result: execution_result, state: ecx.journaled_state.finalize().0 };
    Ok(result)
}
