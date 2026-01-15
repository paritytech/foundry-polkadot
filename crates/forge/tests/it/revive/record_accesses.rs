// NEEDS LATEST BRANCH OF POLKADOT_SDK

// use crate::{config::*, test_helpers::TEST_DATA_REVIVE};
// use foundry_test_utils::Filter;
// use revive_strategy::ReviveRuntimeMode;
// use revm::primitives::hardfork::SpecId;
// use rstest::rstest;
// last failing test: delegate address is incorrect either through the tracer or the pallet-revive
// Traces:
//   [25701] → new <unknown>@0x5F65cD7D792E9746EF82929D60de9a1C526f93A5
//     └─ ← [Return] 128 bytes of code

//   [43119] → new <unknown>@0x57fDDB5Bf9b1660898bEf442fC60ee4924CF6226
//     └─ ← [Return] 215 bytes of code

//   [46722] → new <unknown>@0x4d343D3b8C3c5765746BcfF748AB3E32C6cc37BC
//     └─ ← [Return] 233 bytes of code

//   [44119] → new <unknown>@0x21bCA69b10e0a09bf9557E9eDfdaC03AAD44Bd59
//     └─ ← [Return] 220 bytes of code

//   [41916] → new <unknown>@0x373EbB870b150685555e46A9097965609b787f2d
//     └─ ← [Return] 209 bytes of code

//   [96770] → new <unknown>@0x208a10454580563ED24A5dD56b03F51928A95771
//     └─ ← [Return] 483 bytes of code

//   [77752] → new <unknown>@0xFB6784F8386FDc81FE4eFeFC42e48cb1F33018d9
//     └─ ← [Return] 388 bytes of code

//   [158628] → new <unknown>@0x05F6Dc308173EFcBfB1853f130E466C4A5c60698
//     └─ ← [Return] 792 bytes of code

//   [45122] → new <unknown>@0x8d35bb3497605b2b3d32dcDC311133B120B0A7D8
//     └─ ← [Return] 225 bytes of code

//   [41916] → new <unknown>@0x74c524f11A3943a30ed2459156848932A6B99f64
//     └─ ← [Return] 209 bytes of code

//   [71546] → new <unknown>@0x4E4D0B8A32abfe85Dfa3b54DeFfA75Df2EE4F531
//     └─ ← [Return] 357 bytes of code

//   [41916] → new <unknown>@0xD4B3afcB8Df4D944fDB2F4f72475BC53488e0d66
//     └─ ← [Return] 209 bytes of code

//   [7548718] → new <unknown>@0x7FA9385bE102ac3EAc297483Dd6233D62b3e1496
//     └─ ← [Return] 37584 bytes of code

//   [3020705488] 0x7FA9385bE102ac3EAc297483Dd6233D62b3e1496::setUp()
//     ├─ [3013437920] → new <unknown>@0x5615dEB798BB3E4dFa0139dFa1b3D433Cc23b72f
//     │   └─ ← [Return] 2460 bytes of code
//     ├─ [1439580] → new <unknown>@0xF62849F9A0B5Bf2913b396098F7c7019b51A820a
//     │   └─ ← [Return] 564 bytes of code
//     ├─ [1404204] → new <unknown>@0xc7183455a4C133Ae270771860664b6B7ec320bB1
//     │   └─ ← [Return] 388 bytes of code
//     ├─ [1373853] → new <unknown>@0x1d1499e622D69689cdf9004d05Ec547d650Ff211
//     │   └─ ← [Return] 235 bytes of code
//     ├─ [1373853] → new <unknown>@0x03A6a84cD762D9707A21605b548aaaB891562aAb
//     │   └─ ← [Return] 235 bytes of code
//     ├─ [1366416] → new <unknown>@0x15cF58144EF33af1e14b5208015d11F9143E27b9
//     │   └─ ← [Return] 198 bytes of code
//     └─ ← [Stop]

//   [437184108] 0x7FA9385bE102ac3EAc297483Dd6233D62b3e1496::testStorageAccessDelegateCall()
//     ├─ [175238344] → new <unknown>@0x2a07706473244BC757E10F2a9E86fB532828afe3
//     │   └─ ← [Return] 402 bytes of code
//     ├─ [0] 0x7109709ECfa91a80626fF3989D68f67F5b1DD12D::startStateDiffRecording()
//     │   └─ ← [Return]
//     ├─ [261883196]
// 0x2a07706473244BC757E10F2a9E86fB532828afe3::read(0x00000000000000000000000000000000000000000000000000000000000004d3)
//     │   ├─ [0]
// 0x2a07706473244BC757E10F2a9E86fB532828afe3::read(0x00000000000000000000000000000000000000000000000000000000000004d3)
//     │   │   └─ ← [Return] 0x0000000000000000000000000000000000000000000000000000000000000000
//     │   └─ ← [Return] 0x0000000000000000000000000000000000000000000000000000000000000000
//     ├─ [0] 0x7109709ECfa91a80626fF3989D68f67F5b1DD12D::stopAndReturnStateDiff()
//     │   └─ ← [Return] [AccountAccess({ chainInfo: ChainInfo({ forkId: 0, chainId: 31337 [3.133e4]
// }), kind: 0, account: 0x2a07706473244BC757E10F2a9E86fB532828afe3, accessor:
// 0x7FA9385bE102ac3EAc297483Dd6233D62b3e1496, initialized: true, oldBalance: 0, newBalance: 0,
// deployedCode: 0x, value: 0, data:
// 0x61da143900000000000000000000000000000000000000000000000000000000000004d3, reverted: false,
// storageAccesses: [StorageAccess({ account: 0x2a07706473244BC757E10F2a9E86fB532828afe3, slot:
// 0xe9f62b3e26234094eacd56e2152cac3560ba6d02baf326268344132dbcc553e9, isWrite: false,
// previousValue: 0x0000000000000000000000001d1499e622d69689cdf9004d05ec547d650ff211, newValue:
// 0x0000000000000000000000001d1499e622d69689cdf9004d05ec547d650ff211, reverted: false })], depth: 1
// }), AccountAccess({ chainInfo: ChainInfo({ forkId: 0, chainId: 31337 [3.133e4] }), kind: 1,
// account: 0x2a07706473244BC757E10F2a9E86fB532828afe3, accessor:
// 0x7FA9385bE102ac3EAc297483Dd6233D62b3e1496, initialized: true, oldBalance: 0, newBalance: 0,
// deployedCode: 0x, value: 0, data:
// 0x61da143900000000000000000000000000000000000000000000000000000000000004d3, reverted: false,
// storageAccesses: [StorageAccess({ account: 0x2a07706473244BC757E10F2a9E86fB532828afe3, slot:
// 0x00000000000000000000000000000000000000000000000000000000000004d3, isWrite: false,
// previousValue: 0x0000000000000000000000000000000000000000000000000000000000000000, newValue:
// 0x0000000000000000000000000000000000000000000000000000000000000000, reverted: false })], depth: 2
// })]     ├─ emit log_named_string(key: "Error", val: "incorrect account")
//     ├─ emit log(: "Error: a == b not satisfied [address]")
//     ├─ emit log_named_address(key: "  Expected", val: 0x1d1499e622D69689cdf9004d05Ec547d650Ff211)
//     ├─ emit log_named_address(key: "    Actual", val: 0x2a07706473244BC757E10F2a9E86fB532828afe3)
//     ├─ [0]
// 0x7109709ECfa91a80626fF3989D68f67F5b1DD12D::store(0x7109709ECfa91a80626fF3989D68f67F5b1DD12D,
// 0x6661696c65640000000000000000000000000000000000000000000000000000,
// 0x0000000000000000000000000000000000000000000000000000000000000001)     │   └─ ← [Return]
//     └─ ← [Stop]

// #[rstest]
// #[case::evm(ReviveRuntimeMode::Evm)]
// #[tokio::test(flavor = "multi_thread")]
// async fn test_record_accesses(#[case] runtime_mode: ReviveRuntimeMode) {
//     let runner: forge::MultiContractRunner = TEST_DATA_REVIVE.runner_revive(runtime_mode);
//     let filter = Filter::new(".*", "RecordAccesses", ".*/revive/.*");

//     TestConfig::with_filter(runner, filter).spec_id(SpecId::PRAGUE).run().await;
// }
