use alloy_primitives::{Address, FixedBytes, U256};
use foundry_cheatcodes::Error;
use polkadot_sdk::{
    pallet_revive::{self, AddressMapper},
    sp_core::{self, H160},
    sp_io::TestExternalities,
};
use revive_env::{AccountId, ExtBuilder, Runtime, System, Timestamp};
use std::sync::{Arc, Mutex};
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

impl Clone for TestEnv {
    fn clone(&self) -> Self {
        let mut externalities = ExtBuilder::default().build();
        externalities.backend = self.0.lock().unwrap().as_backend().clone();
        TestEnv(Arc::new(Mutex::new(externalities)))
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

    pub fn set_nonce(&mut self, address: Address, nonce: u64, check_nonce: bool) {
        self.0.lock().unwrap().execute_with(|| {
            let account_id =
                AccountId::to_fallback_account_id(&H160::from_slice(address.as_slice()));
            let current_nonce = System::account_nonce(&account_id);
            if check_nonce {
                assert!(
                    current_nonce as u64 <= nonce,
                    "Cannot set nonce lower than current nonce: {current_nonce} > {nonce}"
                );
            }

            polkadot_sdk::frame_system::Account::<Runtime>::mutate(&account_id, |a| {
                a.nonce = nonce.min(u32::MAX.into()).try_into().expect("shouldn't happen");
            });
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

    pub fn store(
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
