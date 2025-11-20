use alloy_primitives::{Address, Bytes, FixedBytes, U256};
use foundry_cheatcodes::{Ecx, Error, Result};
use polkadot_sdk::{
    pallet_revive::{
        self, AccountInfo, AddressMapper, BalanceOf, BytecodeType, ContractInfo, ExecConfig,
        Executable, Pallet,
    },
    sp_core::{self, H160},
    sp_io::TestExternalities,
};
use revive_env::{AccountId, ExtBuilder, Runtime, System, Timestamp};
use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};
pub struct TestEnv(pub Arc<Mutex<TestExternalities>>);

impl Default for TestEnv {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(
            ExtBuilder::default()
                .balance_genesis_config(vec![(H160::from_low_u64_be(1), 1000)])
                .build(),
        )))
    }
}

impl Debug for TestEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<Externalities>")
    }
}

impl Clone for TestEnv {
    fn clone(&self) -> Self {
        let mut externalities = ExtBuilder::default().build();
        externalities.backend = self.0.lock().unwrap().as_backend();
        Self(Arc::new(Mutex::new(externalities)))
    }
}

impl TestEnv {
    pub fn shallow_clone(&self) -> Self {
        Self(self.0.clone())
    }

    pub fn execute_with<R, F: FnOnce() -> R>(&mut self, f: F) -> R {
        self.0.lock().unwrap().execute_with(f)
    }

    pub fn get_nonce(&mut self, account: Address) -> u32 {
        self.0.lock().unwrap().execute_with(|| {
            System::account_nonce(AccountId::to_fallback_account_id(&H160::from_slice(
                account.as_slice(),
            )))
        })
    }

    pub fn set_nonce(&mut self, address: Address, nonce: u64) {
        self.0.lock().unwrap().execute_with(|| {
            let account_id =
                AccountId::to_fallback_account_id(&H160::from_slice(address.as_slice()));

            polkadot_sdk::frame_system::Account::<Runtime>::mutate(&account_id, |a| {
                a.nonce = nonce.min(u32::MAX.into()).try_into().expect("shouldn't happen");
            });
        });
    }

    pub fn set_chain_id(&mut self, new_chain_id: u64) {
        // Set chain id in pallet-revive runtime.
        self.0.lock().unwrap().execute_with(|| {
            <revive_env::Runtime as polkadot_sdk::pallet_revive::Config>::ChainId::set(
                &new_chain_id,
            );
        });
    }

    pub fn set_block_number(&mut self, new_height: U256) {
        // Set block number in pallet-revive runtime.
        self.0.lock().unwrap().execute_with(|| {
            System::set_block_number(new_height.try_into().expect("Block number exceeds u64"));
        });
    }

    pub fn set_timestamp(&mut self, new_timestamp: U256) {
        // Set timestamp in pallet-revive runtime (milliseconds).
        self.0.lock().unwrap().execute_with(|| {
            let timestamp_ms = new_timestamp.saturating_to::<u64>().saturating_mul(1000);
            Timestamp::set_timestamp(timestamp_ms);
        });
    }

    pub fn etch_call(
        &mut self,
        target: &Address,
        new_runtime_code: &Bytes,
        ecx: Ecx<'_, '_, '_>,
    ) -> Result {
        self.0.lock().unwrap().execute_with(|| {
            let origin_address = H160::from_slice(ecx.tx.caller.as_slice());
            let origin_account = AccountId::to_fallback_account_id(&origin_address);

            let target_address = H160::from_slice(target.as_slice());
            let target_account = AccountId::to_fallback_account_id(&target_address);

            let code = new_runtime_code.to_vec();
            let code_type =
                if code.starts_with(b"PVM\0") { BytecodeType::Pvm } else { BytecodeType::Evm };
            let contract_blob = Pallet::<Runtime>::try_upload_code(
                origin_account,
                code,
                code_type,
                BalanceOf::<Runtime>::MAX,
                &ExecConfig::new_substrate_tx(),
            )
            .map_err(|_| <&str as Into<Error>>::into("Could not upload PVM code"))?
            .0;

            let mut contract_info = if let Some(contract_info) =
                AccountInfo::<Runtime>::load_contract(&target_address)
            {
                contract_info
            } else {
                let contract_info = ContractInfo::<Runtime>::new(
                    &target_address,
                    System::account_nonce(target_account),
                    *contract_blob.code_hash(),
                )
                .map_err(|err| {
                    tracing::error!("Could not create contract info: {:?}", err);
                    <&str as Into<Error>>::into("Could not create contract info")
                })?;
                System::inc_account_nonce(AccountId::to_fallback_account_id(&target_address));
                contract_info
            };
            contract_info.code_hash = *contract_blob.code_hash();
            AccountInfo::<Runtime>::insert_contract(
                &H160::from_slice(target.as_slice()),
                contract_info,
            );
            Ok::<(), Error>(())
        })?;
        Ok(Default::default())
    }

    pub fn get_storage(
        &mut self,
        target: Address,
        slot: FixedBytes<32>,
    ) -> Result<Option<Vec<u8>>, Error> {
        let target_address_h160 = H160::from_slice(target.as_slice());
        self.0
            .lock()
            .unwrap()
            .execute_with(|| {
                pallet_revive::Pallet::<Runtime>::get_storage(target_address_h160, slot.into())
            })
            .map_err(|_| <&str as Into<Error>>::into("Could not set storage"))
    }

    pub fn set_storage(
        &mut self,
        target: Address,
        slot: FixedBytes<32>,
        value: FixedBytes<32>,
    ) -> Result<(), Error> {
        let target_address_h160 = H160::from_slice(target.as_slice());
        self.0
            .lock()
            .unwrap()
            .execute_with(|| {
                pallet_revive::Pallet::<Runtime>::set_storage(
                    target_address_h160,
                    slot.into(),
                    Some(value.to_vec()),
                )
            })
            .map_err(|_| <&str as Into<Error>>::into("Could not set storage"))?;
        Ok(())
    }

    pub fn set_balance(&mut self, address: Address, amount: U256) {
        let amount_pvm =
            sp_core::U256::from_little_endian(&amount.as_le_bytes()).min(u128::MAX.into());

        self.0.lock().unwrap().execute_with(|| {
            let h160_addr = H160::from_slice(address.as_slice());
            pallet_revive::Pallet::<Runtime>::set_evm_balance(&h160_addr, amount_pvm)
                .expect("failed to set evm balance");
        });
    }
    pub fn get_balance(&mut self, address: Address) -> U256 {
        U256::from_limbs(
            self.0
                .lock()
                .unwrap()
                .execute_with(|| {
                    let h160_addr = H160::from_slice(address.as_slice());
                    pallet_revive::Pallet::<Runtime>::evm_balance(&h160_addr)
                })
                .0,
        )
    }
}
