//! Test demonstrating large test contracts work with pallet-revive.
//!
//! Previously, test contracts larger than ~24KB would fail with BlobTooLarge when running
//! `forge test --polkadot`. This was fixed by enabling debug mode in pallet-revive at startup,
//! which bypasses EIP-170 size checks.
//!
//! This test verifies:
//! 1. Test passes in pure EVM (without --polkadot)
//! 2. Test now passes with --polkadot (debug mode enabled, ~28KB test contract)
//! 3. Test passes with --polkadot --resolc (PVM bytecode)

forgetest!(blobtoolarge_callback_morpho_repro, |prj, cmd| {
    prj.insert_ds_test();

    // Simple callback contract (small enough to migrate to PVM)
    prj.add_source(
        "CallbackContract.sol",
        r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

interface ICallback {
    function onCallback(address caller, uint256 value) external returns (bool);
}

contract CallbackContract {
    uint256 public lastValue;
    address public lastCaller;

    function executeWithCallback(uint256 value) public returns (bool) {
        bool authorized = ICallback(msg.sender).onCallback(msg.sender, value);
        if (authorized) {
            lastValue = value;
            lastCaller = msg.sender;
            return true;
        }
        return false;
    }
}
        "#,
    )
    .unwrap();

    // Generate a bloated test contract with many dummy functions to exceed 24KB limit
    let mut bloat_functions = String::new();
    for i in 1..=280 {
        bloat_functions.push_str(&format!(
            "    function bloat{i:03}() public pure returns (uint256) {{ return uint256(keccak256(\"{i:03}\")); }}\n",
        ));
    }

    let test_contract = format!(
        r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import "./test.sol";
import "./CallbackContract.sol";

contract BloatTest is DSTest, ICallback {{
    function onCallback(address, uint256 value) external pure returns (bool) {{
        return value < 1000;
    }}

    function testCallback() public {{
        CallbackContract c = new CallbackContract();
        bool result = c.executeWithCallback(500);
        assertTrue(result);
        assertEq(c.lastValue(), 500);
    }}

{bloat_functions}
}}
        "#
    );

    prj.add_source("BloatTest.t.sol", &test_contract).unwrap();

    // Without --polkadot, test runs in pure EVM and passes
    cmd.forge_fuse().args(["test", "--match-test", "testCallback"]).assert_success();

    // With --polkadot, test now passes with large test contract (~28KB)
    // Debug mode is enabled at startup to bypass EIP-170 size checks
    cmd.forge_fuse().args(["test", "--polkadot", "--match-test", "testCallback"]).assert_success();

    // Also works with --resolc (compiles to PVM bytecode)
    cmd.forge_fuse()
        .args(["test", "--polkadot", "--resolc", "--match-test", "testCallback"])
        .assert_success();
});
