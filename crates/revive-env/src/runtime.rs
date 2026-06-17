//! Dummy runtime with pallet revive and all necessary pallets for running tests with forge.
//!
//! THIS IS WORK IN PROGRESS. It is not yet complete and may change in the future.

use frame_support::{runtime, traits::FindAuthor, weights::constants::WEIGHT_REF_TIME_PER_SECOND};
use polkadot_sdk::{
    frame_support::weights::constants::{BlockExecutionWeight, ExtrinsicBaseWeight},
    pallet_revive::{
        AccountId32Mapper, BlockWeights,
        evm::{
            fees::{BlockRatioFee, Info},
            runtime::EthExtra,
        },
    },
    pallet_transaction_payment::{ConstFeeMultiplier, Multiplier},
    parachains_common::{AccountId, Hash, Nonce},
    polkadot_sdk_frame::runtime::prelude::*,
    sp_runtime::{AccountId32, generic},
    sp_weights::ConstantMultiplier,
    *,
};

pub type Balance = u128;
pub type Block = sp_runtime::generic::Block<generic::Header<u64, BlakeTwo256>, UncheckedExtrinsic>;
pub type UncheckedExtrinsic =
    pallet_revive::evm::runtime::UncheckedExtrinsic<Address, Signature, EthExtraImpl>;

pub mod currency {
    use super::Balance;
    pub const DOLLARS: Balance = 1_000_000_000_000;
    pub const CENTS: Balance = DOLLARS / 100;
    pub const MILLICENTS: Balance = CENTS / 1_000;
}

parameter_types! {
    pub const TransactionByteFee: Balance = 10 * currency::MILLICENTS;
    pub FeeMultiplier: Multiplier = Multiplier::one();
}

// Implements the types required for the transaction payment pallet.
#[derive_impl(pallet_transaction_payment::config_preludes::TestDefaultConfig)]
impl pallet_transaction_payment::Config for Runtime {
    type OnChargeTransaction = pallet_transaction_payment::FungibleAdapter<Balances, ()>;
    type WeightToFee = BlockRatioFee<1, 1, Self, Balance>;
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
    type AccountId = AccountId;
    type Hash = Hash;
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

/// We assume that ~10% of the block weight is consumed by `on_initialize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
#[allow(unused)]
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
#[allow(unused)]
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 2 seconds of compute with a 6 second average block time, with maximum proof size.
#[allow(unused)]
const MAXIMUM_BLOCK_WEIGHT: Weight =
    Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2), u64::MAX);

parameter_types! {
    pub const UnstableInterface: bool = true;
    pub const CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(0);
    pub const NativeToEthRatio: u32 = 1_000_000;
    pub const GasScale : u32 = 100_000_000;

    pub const DepositPerByte: Balance = 1;
    pub const DepositPerItem: Balance = 2;
    pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
    .base_block(BlockExecutionWeight::get())
    .for_class(DispatchClass::all(), |weights| {
        weights.base_extrinsic = ExtrinsicBaseWeight::get();
    })
    .for_class(DispatchClass::Normal, |weights| {
        weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
    })
    .for_class(DispatchClass::Operational, |weights| {
        weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
        // Operational transactions have some extra reserved space, so that they
        // are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
        weights.reserved = Some(
            MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
        );
    })
    .avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
    .build_or_panic();
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
    type UploadOrigin = EnsureSigned<AccountId32>;
    type InstantiateOrigin = EnsureSigned<AccountId32>;
    type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
    type ChainId = ChainId;
    type NativeToEthRatio = NativeToEthRatio;
    type FindAuthor = Self;
    type DebugEnabled = ConstBool<true>;
    type GasScale = GasScale;
    type FeeInfo = Info<Address, Signature, EthExtraImpl>;
}

pub use polkadot_sdk::parachains_common::Signature;

pub type Address = sp_runtime::MultiAddress<AccountId, ()>;

/// The transaction extensions that are added to the runtime.
type TxExtension = (
    // Checks that the sender is not the zero address.
    frame_system::CheckNonZeroSender<Runtime>,
    // Checks that the runtime version is correct.
    frame_system::CheckSpecVersion<Runtime>,
    // Checks that the transaction version is correct.
    frame_system::CheckTxVersion<Runtime>,
    // Checks that the genesis hash is correct.
    frame_system::CheckGenesis<Runtime>,
    // Checks that the era is valid.
    frame_system::CheckEra<Runtime>,
    // Checks that the nonce is valid.
    frame_system::CheckNonce<Runtime>,
    // Checks that the weight is valid.
    frame_system::CheckWeight<Runtime>,
    // Ensures that the sender has enough funds to pay for the transaction
    // and deducts the fee from the sender's account.
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
    // Needs to be done after all extensions that rely on a signed origin.
    pallet_revive::evm::tx_extension::SetOrigin<Runtime>,
    // Reclaim the unused weight from the block using post dispatch information.
    // It must be last in the pipeline in order to catch the refund in previous transaction
    // extensions
    frame_system::WeightReclaim<Runtime>,
);
/// Default extensions applied to Ethereum transactions.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EthExtraImpl;

impl EthExtra for EthExtraImpl {
    type Config = Runtime;
    type ExtensionV0 = TxExtension;
    type ExtensionOtherVersions = sp_runtime::traits::InvalidVersion;

    fn get_eth_extension(nonce: u32, tip: Balance) -> Self::ExtensionV0 {
        (
            frame_system::CheckNonZeroSender::<Runtime>::new(),
            frame_system::CheckSpecVersion::<Runtime>::new(),
            frame_system::CheckTxVersion::<Runtime>::new(),
            frame_system::CheckGenesis::<Runtime>::new(),
            frame_system::CheckMortality::from(sp_runtime::generic::Era::Immortal),
            frame_system::CheckNonce::<Runtime>::from(nonce),
            frame_system::CheckWeight::<Runtime>::new(),
            pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
            pallet_revive::evm::tx_extension::SetOrigin::<Runtime>::new_from_eth_transaction(),
            frame_system::WeightReclaim::<Runtime>::new(),
        )
    }
}

impl pallet_revive::evm::runtime::SetWeightLimit for RuntimeCall {
    fn set_weight_limit(&mut self, new_weight_limit: Weight) -> Weight {
        use pallet_revive::pallet::Call as ReviveCall;
        match self {
            Self::Contracts(
                ReviveCall::eth_call { weight_limit, .. }
                | ReviveCall::eth_instantiate_with_code { weight_limit, .. },
            ) => {
                let old = *weight_limit;
                *weight_limit = new_weight_limit;
                old
            }
            _ => Weight::default(),
        }
    }
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
