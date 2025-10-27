use polkadot_sdk::{
    sc_consensus::BlockImportParams,
    sc_consensus_aura::CompatibleDigestItem,
    sc_consensus_manual_seal::{ConsensusDataProvider, Error},
    sp_consensus_aura::ed25519::AuthoritySignature,
    sp_consensus_babe::Slot,
    sp_inherents::InherentData,
    sp_runtime::{Digest, DigestItem, traits::Block as BlockT},
};
use std::marker::PhantomData;

/// Consensus data provider for Aura. This will always use slot 0 (used to determine the
/// index of the AURA authority from the authorities set by AURA runtimes) for the aura
/// digest since anvil-polkadot node will be the sole block author and AURA will pick
/// only its configured address, residing at index 0 in the AURA authorities set. When
/// forking from an assethub chain, we expect an assethub runtime based on AURA,
/// which will pick the author based on the slot given through the digest, which will
/// also result in picking the AURA authority from index 0.
pub struct SameSlotConsensusDataProvider<B, P> {
    _phantom: PhantomData<(B, P)>,
}

impl<B, P> SameSlotConsensusDataProvider<B, P> {
    pub fn new() -> Self {
        Self { _phantom: PhantomData }
    }
}

impl<B, P> ConsensusDataProvider<B> for SameSlotConsensusDataProvider<B, P>
where
    B: BlockT,
    P: Send + Sync,
{
    type Proof = P;

    fn create_digest(
        &self,
        _parent: &B::Header,
        _inherents: &InherentData,
    ) -> Result<Digest, Error> {
        let digest_item = <DigestItem as CompatibleDigestItem<AuthoritySignature>>::aura_pre_digest(
            Slot::default(),
        );

        Ok(Digest { logs: vec![digest_item] })
    }

    fn append_block_import(
        &self,
        _parent: &B::Header,
        _params: &mut BlockImportParams<B>,
        _inherents: &InherentData,
        _proof: Self::Proof,
    ) -> Result<(), Error> {
        Ok(())
    }
}
