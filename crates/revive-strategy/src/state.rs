use alloy_primitives::{Address, B256, Bytes, FixedBytes, U256};
use foundry_cheatcodes::{Error, Result};
use polkadot_sdk::{
    frame_support::traits::{
        fungible::{InspectHold, MutateHold},
        tokens::Precision,
    },
    pallet_revive::{
        self, AccountId32Mapper, AccountInfo, AddressMapper, BytecodeType, ContractInfo,
        ExecConfig, Executable, HoldReason, Pallet, ResourceMeter,
    },
    sp_core::{self, H160, H256},
    sp_externalities::Externalities,
    sp_io::TestExternalities,
    sp_runtime::AccountId32,
    sp_weights::Weight,
};
use revive_env::{Balances, BlockAuthor, ExtBuilder, NativeToEthRatio, Runtime, System, Timestamp};
use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

pub(crate) struct Inner {
    pub externalities: TestExternalities,
    pub depth: usize,
}

#[derive(Default)]
pub struct TestEnv(pub(crate) Arc<Mutex<Inner>>);

impl Default for Inner {
    fn default() -> Self {
        Self {
            externalities: ExtBuilder::default()
                .balance_genesis_config(vec![(
                    H160::from_low_u64_be(1),
                    1_000_000_000_000_000_000_000_000_000_u128,
                )])
                .build(),
            depth: 0,
        }
    }
}

impl Debug for TestEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<Externalities>")
    }
}

impl Clone for TestEnv {
    fn clone(&self) -> Self {
        let mut inner: Inner = Default::default();
        inner.externalities.backend = self.0.lock().unwrap().externalities.as_backend();
        inner.depth = self.0.lock().unwrap().depth;
        Self(Arc::new(Mutex::new(inner)))
    }
}

impl TestEnv {
    pub fn shallow_clone(&self) -> Self {
        Self(self.0.clone())
    }

    pub fn start_snapshotting(&mut self) {
        let mut state = self.0.lock().unwrap();
        state.depth += 1;
        state.externalities.ext().storage_start_transaction();
    }

    pub fn revert(&mut self, depth: usize) {
        let mut state = self.0.lock().unwrap();
        while state.depth > depth + 1 {
            state.externalities.ext().storage_rollback_transaction().unwrap();
            state.depth -= 1;
        }
        state.externalities.ext().storage_rollback_transaction().unwrap();
        state.externalities.ext().storage_start_transaction();
    }

    pub fn execute_with<R, F: FnOnce() -> R>(&mut self, f: F) -> R {
        self.0.lock().unwrap().externalities.execute_with(f)
    }

    pub fn get_nonce(&mut self, account: Address) -> u32 {
        self.0.lock().unwrap().externalities.execute_with(|| {
            System::account_nonce(AccountId32Mapper::<Runtime>::to_fallback_account_id(
                &H160::from_slice(account.as_slice()),
            ))
        })
    }

    pub fn set_nonce(&mut self, address: Address, nonce: u64) {
        self.0.lock().unwrap().externalities.execute_with(|| {
            let account_id = AccountId32Mapper::<Runtime>::to_fallback_account_id(
                &H160::from_slice(address.as_slice()),
            );

            polkadot_sdk::frame_system::Account::<Runtime>::mutate(&account_id, |a| {
                a.nonce = nonce.min(u32::MAX.into()).try_into().expect("shouldn't happen");
            });
        });
    }

    pub fn set_chain_id(&mut self, new_chain_id: u64) {
        // Set chain id in pallet-revive runtime.
        self.0.lock().unwrap().externalities.execute_with(|| {
            <revive_env::Runtime as polkadot_sdk::pallet_revive::Config>::ChainId::set(
                &new_chain_id,
            );
        });
    }

    pub fn set_block_number(
        &mut self,
        new_height: U256,
        prev_new_height_hash: B256,
        new_height_hash: B256,
    ) -> U256 {
        // Set block number in pallet-revive runtime.
        self.0.lock().unwrap().externalities.execute_with(|| {
            let u64_max = U256::from(u64::MAX);
            let clamped_height = if new_height > u64_max {
                tracing::warn!(
                    block_number = ?new_height,
                    max = ?u64_max,
                    "Block number exceeds u64::MAX. Clamping to u64::MAX."
                );
                u64_max
            } else {
                new_height
            };

            let new_block_number: u64 = clamped_height.to();
            let digest = System::digest();
            if System::block_hash(new_block_number) == H256::zero() {
                // First initialize and finalize the parent block to set up correct hashes.
                if new_block_number > 0 {
                    System::set_block_number(new_block_number - 1);
                    let current_hash = H256::from_slice(prev_new_height_hash.0.as_slice());
                    System::initialize(&new_block_number, &current_hash, &digest);
                }

                // Now finalize the new block to set up its hash.
                if new_block_number < u64::MAX {
                    System::set_block_number(new_block_number);
                    let current_hash = H256::from_slice(new_height_hash.0.as_slice());
                    System::initialize(&(new_block_number + 1), &current_hash, &digest);
                }
            }
            System::set_block_number(new_block_number);
            clamped_height
        })
    }

    pub fn get_block_number(&mut self) -> U256 {
        // Get block number in pallet-revive runtime.
        self.0.lock().unwrap().externalities.execute_with(|| U256::from(System::block_number()))
    }

    pub fn roll<DB: revm::Database + ?Sized>(
        &mut self,
        new_height: U256,
        database: &mut DB,
    ) -> U256 {
        let block_num_u64 = new_height.saturating_to::<u64>();
        let prev_block_hash =
            database.block_hash(block_num_u64.saturating_sub(1)).unwrap_or_default();
        let current_block_hash = database.block_hash(block_num_u64).unwrap_or_default();

        self.set_block_number(new_height, prev_block_hash, current_block_hash)
    }

    pub fn set_timestamp(&mut self, new_timestamp: U256) -> U256 {
        // Set timestamp in pallet-revive runtime (milliseconds).
        self.0.lock().unwrap().externalities.execute_with(|| {
            let u64_max = U256::from(u64::MAX);
            let clamped_timestamp = if new_timestamp > u64_max {
                tracing::warn!(
                    timestamp = ?new_timestamp,
                    max = ?u64_max,
                    "Timestamp exceeds u64::MAX. Clamping to u64::MAX."
                );
                u64_max
            } else {
                new_timestamp
            };

            let timestamp_ms = clamped_timestamp.saturating_to::<u64>().saturating_mul(1000);
            Timestamp::set_timestamp(timestamp_ms);
            clamped_timestamp
        })
    }

    fn set_base_deposit_hold(
        target_address: &H160,
        target_account: &AccountId32,
        contract_info: &mut ContractInfo<Runtime>,
        code_deposit: u128,
    ) -> foundry_cheatcodes::Result {
        contract_info.update_base_deposit(code_deposit);

        let base_deposit: u128 = contract_info.storage_base_deposit();
        let hold_reason: revive_env::RuntimeHoldReason = HoldReason::StorageDepositReserve.into();

        // Release any existing hold
        let current_held = Balances::balance_on_hold(&hold_reason, target_account);
        if current_held > 0 {
            Balances::release(&hold_reason, target_account, current_held, Precision::BestEffort)
                .map_err(|_| <&str as Into<Error>>::into("Could not release old hold"))?;

            // Decrease EVM balance by released amount (hold became free, so visible balance would
            // increase)
            let current_evm_balance = Pallet::<Runtime>::evm_balance(target_address);
            let release_wei = sp_core::U256::from(current_held)
                .saturating_mul(sp_core::U256::from(NativeToEthRatio::get() as u128));
            let adjusted_balance = current_evm_balance.saturating_sub(release_wei);
            Pallet::<Runtime>::set_evm_balance(target_address, adjusted_balance).map_err(|_| {
                <&str as Into<Error>>::into("Could not adjust balance after release")
            })?;
        }

        // Create new hold with correct amount
        if base_deposit > 0 {
            let current_evm_balance = Pallet::<Runtime>::evm_balance(target_address);
            let hold_wei = sp_core::U256::from(base_deposit)
                .saturating_mul(sp_core::U256::from(NativeToEthRatio::get() as u128));
            let new_evm_balance = current_evm_balance.saturating_add(hold_wei);

            Pallet::<Runtime>::set_evm_balance(target_address, new_evm_balance)
                .map_err(|_| <&str as Into<Error>>::into("Could not set balance for new hold"))?;

            Balances::hold(&hold_reason, target_account, base_deposit)
                .map_err(|_| <&str as Into<Error>>::into("Could not create new hold"))?;
        }

        Ok(Default::default())
    }

    pub fn etch_call(&mut self, target: &Address, new_runtime_code: &Bytes) -> Result {
        self.0.lock().unwrap().externalities.execute_with(|| {
            let target_address = H160::from_slice(target.as_slice());
            let target_account =
                AccountId32Mapper::<Runtime>::to_fallback_account_id(&target_address);

            let code = new_runtime_code.to_vec();
            let code_type =
                if code.starts_with(b"PVM\0") { BytecodeType::Pvm } else { BytecodeType::Evm };
            let contract_blob = Pallet::<Runtime>::try_upload_code(
                Pallet::<Runtime>::account_id(),
                code,
                code_type,
                &mut ResourceMeter::new(pallet_revive::TransactionLimits::WeightAndDeposit {
                    weight_limit: Weight::from_parts(10_000_000_000_000, 100_000_000),
                    deposit_limit: { 100_000_000_000_000 },
                })
                .unwrap(),
                &ExecConfig::new_substrate_tx(),
            )
            .map_err(|_| <&str as Into<Error>>::into("Could not upload PVM code"))?;

            let code_deposit = contract_blob.code_info().deposit();
            let code_hash = *contract_blob.code_hash();

            let mut contract_info = if let Some(contract_info) =
                AccountInfo::<Runtime>::load_contract(&target_address)
            {
                contract_info
            } else {
                let contract_info = ContractInfo::<Runtime>::new(
                    &target_address,
                    System::account_nonce(&target_account),
                    code_hash,
                )
                .map_err(|err| {
                    tracing::error!("Could not create contract info: {:?}", err);
                    <&str as Into<Error>>::into("Could not create contract info")
                })?;
                System::inc_account_nonce(AccountId32Mapper::<Runtime>::to_fallback_account_id(
                    &target_address,
                ));
                contract_info
            };

            contract_info.code_hash = code_hash;

            // Update base deposit hold for both new and existing contracts
            // Note: Code upload deposits are already held on the pallet account by try_upload_code
            Self::set_base_deposit_hold(
                &target_address,
                &target_account,
                &mut contract_info,
                code_deposit,
            )?;

            AccountInfo::<Runtime>::insert_contract(
                &H160::from_slice(target.as_slice()),
                contract_info.clone(),
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
            .externalities
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
            .externalities
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

    pub fn set_balance(&mut self, address: Address, amount: U256) -> U256 {
        let u128_max = U256::from(u128::MAX);
        let clamped_amount = if amount > u128_max {
            tracing::warn!(
                address = ?address,
                requested = ?amount,
                actual = ?u128_max,
                "Balance exceeds u128::MAX, clamping to u128::MAX. \
                 pallet-revive uses u128 for balances."
            );
            u128_max
        } else {
            amount
        };

        let amount_pvm = sp_core::U256::from_little_endian(&clamped_amount.as_le_bytes());

        self.0.lock().unwrap().externalities.execute_with(|| {
            let h160_addr = H160::from_slice(address.as_slice());
            pallet_revive::Pallet::<Runtime>::set_evm_balance(&h160_addr, amount_pvm)
                .expect("failed to set evm balance");
        });

        clamped_amount
    }

    pub fn get_balance(&mut self, address: Address) -> U256 {
        U256::from_limbs(
            self.0
                .lock()
                .unwrap()
                .externalities
                .execute_with(|| {
                    let h160_addr = H160::from_slice(address.as_slice());
                    pallet_revive::Pallet::<Runtime>::evm_balance(&h160_addr)
                })
                .0,
        )
    }

    pub fn set_block_author(&mut self, new_author: Address) {
        self.0.lock().unwrap().externalities.execute_with(|| {
            let account_id32 = AccountId32Mapper::<Runtime>::to_fallback_account_id(
                &H160::from_slice(new_author.as_slice()),
            );
            BlockAuthor::set(&account_id32);
        });
    }

    pub fn set_blockhash(&mut self, block_number: u64, block_hash: FixedBytes<32>) {
        self.0.lock().unwrap().externalities.execute_with(|| {
            use polkadot_sdk::frame_system::BlockHash;

            let hash = sp_core::H256::from_slice(block_hash.as_slice());
            BlockHash::<Runtime>::insert::<u64, _>(block_number, hash);
        });
    }

    pub fn is_contract(&self, address: Address) -> bool {
        self.0.lock().unwrap().externalities.execute_with(|| {
            AccountInfo::<Runtime>::load_contract(&H160::from_slice(address.as_slice())).is_some()
        })
    }
}
