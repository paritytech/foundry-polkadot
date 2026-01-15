use crate::api_server::{
    error::{Error, Result},
    revive_conversions::ReviveAddress,
};
use alloy_dyn_abi::TypedData;
use alloy_primitives::{Address, utils::eip191_hash_message};
use polkadot_sdk::pallet_revive::evm::{Account, TransactionSigned, TransactionUnsigned};
use std::collections::HashMap;
use subxt::utils::H160;
use subxt_signer::eth::Keypair;

pub struct DevSigner {
    keypairs: HashMap<H160, Keypair>,
}

impl DevSigner {
    pub fn new(private_keys: Vec<Keypair>) -> Result<Self> {
        let keypairs: HashMap<H160, Keypair> =
            private_keys.into_iter().map(|kp| (Account::from(kp.clone()).address(), kp)).collect();
        Ok(Self { keypairs })
    }

    fn recovery_id_mapper(id: u8) -> u8 {
        id + 27
    }

    pub(crate) fn accounts(&self) -> Vec<H160> {
        self.keypairs.keys().copied().collect()
    }

    pub(crate) fn sign_transaction(
        &self,
        address: Address,
        transaction: TransactionUnsigned,
    ) -> Result<TransactionSigned> {
        let keypair = self
            .keypairs
            .get(&ReviveAddress::from(address).inner())
            .ok_or(Error::NoSignerAvailable)?;
        let account = Account::from(keypair.clone());
        Ok(account.sign_transaction(transaction))
    }

    pub(crate) fn sign(&self, address: Address, message: &[u8]) -> Result<[u8; 65]> {
        let keypair = self
            .keypairs
            .get(&ReviveAddress::from(address).inner())
            .ok_or(Error::NoSignerAvailable)?;
        let hash = eip191_hash_message(message);
        let mut signature = keypair.sign_prehashed(hash.as_ref()).0;
        signature[64] = Self::recovery_id_mapper(signature[64]);
        Ok(signature)
    }

    pub(crate) fn sign_typed_data(
        &self,
        address: Address,
        typed_data: &TypedData,
    ) -> Result<[u8; 65]> {
        let keypair = self
            .keypairs
            .get(&ReviveAddress::from(address).inner())
            .ok_or(Error::NoSignerAvailable)?;

        // Compute the EIP-712 signing hash
        let hash =
            typed_data.eip712_signing_hash().map_err(|e| Error::InternalError(e.to_string()))?;
        let mut signature = keypair.sign_prehashed(hash.as_ref()).0;
        signature[64] = Self::recovery_id_mapper(signature[64]);
        Ok(signature)
    }
}
