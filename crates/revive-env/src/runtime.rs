//! Dummy runtime with pallet revive and all necessary pallets for running tests with forge.
//!
//! THIS IS WORK IN PROGRESS. It is not yet complete and may change in the future.

use frame_support::{runtime, traits::FindAuthor, weights::constants::WEIGHT_REF_TIME_PER_SECOND};
use pallet_revive::AccountId32Mapper;
use polkadot_sdk::{
    pallet_revive::evm::{
        fees::{BlockRatioFee, Info as FeeInfo},
        runtime::EthExtra,
    },
    pallet_transaction_payment::{ConstFeeMultiplier, Multiplier},
    polkadot_sdk_frame::{
        runtime::{apis, prelude::*},
        traits::Block as BlockT,
    },
    sp_runtime::{
        traits::{Lazy, Verify},
        AccountId32,
    },
    sp_weights::ConstantMultiplier,
    *,
};

pub type Balance = u128;
pub type BlockNumber = u64;
type Header = sp_runtime::generic::Header<BlockNumber, BlakeTwo256>;
pub type Block = sp_runtime::generic::Block<
    Header,
    frame_system::mocking::MockUncheckedExtrinsic<Runtime, MockSignature, TxExtension>,
>;
pub type Nonce = u32;
pub type AccountId = AccountId32;
pub type AccountIdMapper = pallet_revive::AccountId32Mapper<Runtime>;

#[derive(
    PartialEq,
    Eq,
    Clone,
    Encode,
    Decode,
    DecodeWithMemTracking,
    Debug,
    Hash,
    PartialOrd,
    Ord,
    MaxEncodedLen,
    TypeInfo,
)]
pub struct MockSignature(AccountId32);

impl IdentifyAccount for MockSignature {
    type AccountId = AccountId32;

    fn into_account(self) -> Self::AccountId {
        self.0
    }
}

impl Verify for MockSignature {
    type Signer = Self;

    fn verify<L: Lazy<[u8]>>(
        &self,
        _msg: L,
        _signer: &<Self::Signer as IdentifyAccount>::AccountId,
    ) -> bool {
        true
    }
}

parameter_types! {
    pub const TransactionByteFee: Balance = 10;
    pub FeeMultiplier: Multiplier = Multiplier::one();
}

// Implements the types required for the transaction payment pallet.
#[derive_impl(pallet_transaction_payment::config_preludes::TestDefaultConfig)]
impl pallet_transaction_payment::Config for Runtime {
    type OnChargeTransaction = pallet_transaction_payment::FungibleAdapter<Balances, ()>;
    type WeightToFee = BlockRatioFee<1, 1, Self>;
    type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
    type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
}

#[runtime]
mod runtime {
    #[runtime::runtime]
    #[runtime::derive(
        RuntimeCall,
        RuntimeEvent,
        RuntimeError,
        RuntimeOrigin,
        RuntimeFreezeReason,
        RuntimeHoldReason,
        RuntimeSlashReason,
        RuntimeLockId,
        RuntimeTask,
        RuntimeViewFunction
    )]
    pub struct Runtime;

    #[runtime::pallet_index(0)]
    pub type System = frame_system;

    #[runtime::pallet_index(1)]
    pub type Timestamp = pallet_timestamp;

    #[runtime::pallet_index(2)]
    pub type Balances = pallet_balances;

    #[runtime::pallet_index(3)]
    pub type Contracts = pallet_revive;

    /// Provides the ability to charge for extrinsic execution.
    #[runtime::pallet_index(4)]
    pub type TransactionPayment = pallet_transaction_payment::Pallet<Runtime>;
}

#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig)]
impl frame_system::Config for Runtime {
    type Block = Block;
    type BlockWeights = BlockWeights;
    type AccountId = AccountId32;
    type Lookup = IdentityLookup<AccountId32>;
    type Nonce = Nonce;
    type AccountData = pallet_balances::AccountData<<Self as pallet_balances::Config>::Balance>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Runtime {
    type AccountStore = System;
    type Balance = Balance;
    type ExistentialDeposit = ConstU128<1_000>;
}

#[derive_impl(pallet_timestamp::config_preludes::TestDefaultConfig)]
impl pallet_timestamp::Config for Runtime {}

parameter_types! {
    pub const UnstableInterface: bool = true;
    pub const DepositPerByte: Balance = 1;
    pub const DepositPerItem: Balance = 1;
    pub const CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(0);
    pub const NativeToEthRatio: u32 = 1_000_000;
    pub const GasScale : u32 = 1_000_000;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(
            Weight::from_parts(2u64 * WEIGHT_REF_TIME_PER_SECOND, u64::MAX),
        );
}

type TxExtension = (pallet_transaction_payment::ChargeTransactionPayment<Runtime>,);

/// Default extensions applied to Ethereum transactions.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EthExtraImpl;

impl EthExtra for EthExtraImpl {
    type Config = Runtime;
    type Extension = TxExtension;

    fn get_eth_extension(_nonce: u32, tip: Balance) -> Self::Extension {
        (pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),)
    }
}

#[derive_impl(pallet_revive::config_preludes::TestDefaultConfig)]
impl pallet_revive::Config for Runtime {
    type Time = Timestamp;
    type Balance = Balance;
    type Currency = Balances;
    type DepositPerByte = DepositPerByte;
    type DepositPerItem = DepositPerItem;
    type AddressMapper = AccountId32Mapper<Self>;
    type RuntimeMemory = ConstU32<{ 512 * 1024 * 1024 }>;
    type PVFMemory = ConstU32<{ 1024 * 1024 * 1024 }>;
    type UnsafeUnstableInterface = UnstableInterface;
    type UploadOrigin = EnsureSigned<AccountId32>;
    type InstantiateOrigin = EnsureSigned<AccountId32>;
    type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
    type ChainId = ChainId;
    type FeeInfo =
        FeeInfo<<Runtime as frame_system::Config>::AccountId, MockSignature, EthExtraImpl>;
    type NativeToEthRatio = NativeToEthRatio;
    type FindAuthor = Self;
    type DebugEnabled = ConstBool<true>;
    type GasScale = GasScale;
}

parameter_types! {
    pub storage ChainId: u64 = 420_420_420;
    pub storage BlockAuthor: AccountId32 = {
        [[0xff; 20].as_slice(), [0xee; 12].as_slice()].concat().as_slice().try_into().unwrap()
    };
}

impl FindAuthor<<Self as frame_system::Config>::AccountId> for Runtime {
    fn find_author<'a, I>(_digests: I) -> Option<<Self as frame_system::Config>::AccountId>
    where
        I: 'a + IntoIterator<Item = (frame_support::ConsensusEngineId, &'a [u8])>,
    {
        Some(BlockAuthor::get())
    }
}

extern crate alloc;

type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
>;

/// The runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: alloc::borrow::Cow::Borrowed("revive-dev-runtime"),
    impl_name: alloc::borrow::Cow::Borrowed("revive-dev-runtime"),
    authoring_version: 1,
    spec_version: 0,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
    system_version: 1,
};

pallet_revive::impl_runtime_apis_plus_revive_traits!(Runtime, Contracts, Executive, EthExtraImpl,
    impl apis::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(_block: <Block as BlockT>::LazyBlock) {

        }

        fn initialize_block(_header: &Header) -> ExtrinsicInclusionMode {
            ExtrinsicInclusionMode::AllExtrinsics
        }
    }
);
