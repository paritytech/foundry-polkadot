//! Tests for the `forge compiler` command.

use foundry_test_utils::snapbox::IntoData;

const CONTRACT_A: &str = r#"
// SPDX-license-identifier: MIT
pragma solidity 0.8.4;

contract ContractA {}
"#;

const CONTRACT_B: &str = r#"
// SPDX-license-identifier: MIT
pragma solidity 0.8.11;

contract ContractB {}
"#;

const CONTRACT_C: &str = r#"
// SPDX-license-identifier: MIT
pragma solidity 0.8.27;

contract ContractC {}
"#;

const CONTRACT_D: &str = r#"
// SPDX-license-identifier: MIT
pragma solidity 0.8.27;

contract ContractD {}
"#;

const VYPER_INTERFACE: &str = r#"
# pragma version >=0.4.0

@external
@view
def number() -> uint256:
    return empty(uint256)

@external
def set_number(new_number: uint256):
    pass

@external
def increment() -> uint256:
    return empty(uint256)
"#;

const VYPER_CONTRACT: &str = r#"
import ICounter
implements: ICounter

number: public(uint256)

@external
def set_number(new_number: uint256):
    self.number = new_number

@external
def increment() -> uint256:
    self.number += 1
    return self.number
"#;

forgetest!(can_resolve_path, |prj, cmd| {
    prj.add_source("ContractA", CONTRACT_A).unwrap();

    cmd.args(["compiler", "resolve", "--root", prj.root().to_str().unwrap()])
        .assert_success()
        .stdout_eq(str![[r#"
Solidity:
- Solc v0.8.4


"#]]);
});

forgetest!(can_list_resolved_compiler_versions, |prj, cmd| {
    prj.add_source("ContractA", CONTRACT_A).unwrap();

    cmd.args(["compiler", "resolve"]).assert_success().stdout_eq(str![[r#"
Solidity:
- Solc v0.8.4


"#]]);
});

forgetest!(can_list_resolved_compiler_versions_json, |prj, cmd| {
    prj.add_source("ContractA", CONTRACT_A).unwrap();

    cmd.args(["compiler", "resolve", "--json"]).assert_success().stdout_eq(
        str![[r#"
{
  "Solidity": [
    {
      "name": "Solc",
      "version": "0.8.4"
    }
  ]
}
"#]]
        .is_json(),
    );
});

forgetest!(can_list_resolved_compiler_versions_verbose, |prj, cmd| {
    prj.add_source("ContractC", CONTRACT_C).unwrap();
    prj.add_source("ContractD", CONTRACT_D).unwrap();

    cmd.args(["compiler", "resolve", "-v"]).assert_success().stdout_eq(str![[r#"
Solidity:

Solc v0.8.27:
├── src/ContractC.sol
└── src/ContractD.sol


"#]]);
});

forgetest!(can_list_resolved_compiler_versions_verbose_json, |prj, cmd| {
    prj.add_source("ContractC", CONTRACT_C).unwrap();
    prj.add_source("ContractD", CONTRACT_D).unwrap();

    cmd.args(["compiler", "resolve", "--json", "-v"]).assert_success().stdout_eq(
        str![[r#"
{
  "Solidity": [
    {
      "name": "Solc",
      "version": "0.8.27",
      "paths": [
        "src/ContractC.sol",
        "src/ContractD.sol"
      ]
    }
  ]
}
"#]]
        .is_json(),
    );
});

forgetest!(can_list_resolved_multiple_compiler_versions, |prj, cmd| {
    prj.add_source("ContractA", CONTRACT_A).unwrap();
    prj.add_source("ContractB", CONTRACT_B).unwrap();
    prj.add_source("ContractC", CONTRACT_C).unwrap();
    prj.add_source("ContractD", CONTRACT_D).unwrap();
    prj.add_raw_source("ICounter.vyi", VYPER_INTERFACE).unwrap();
    prj.add_raw_source("Counter.vy", VYPER_CONTRACT).unwrap();

    cmd.args(["compiler", "resolve"]).assert_success().stdout_eq(str![[r#"
Solidity:
- Solc v0.8.4
- Solc v0.8.11
- Solc v0.8.27

Vyper:
- Vyper v0.4.0


"#]]);
});

forgetest!(can_list_resolved_multiple_compiler_versions_skipped, |prj, cmd| {
    prj.add_source("ContractA", CONTRACT_A).unwrap();
    prj.add_source("ContractB", CONTRACT_B).unwrap();
    prj.add_source("ContractC", CONTRACT_C).unwrap();
    prj.add_source("ContractD", CONTRACT_D).unwrap();
    prj.add_raw_source("ICounter.vyi", VYPER_INTERFACE).unwrap();
    prj.add_raw_source("Counter.vy", VYPER_CONTRACT).unwrap();

    cmd.args(["compiler", "resolve", "--skip", ".sol", "-v"]).assert_success().stdout_eq(str![[
        r#"
Vyper:

Vyper v0.4.0:
├── src/Counter.vy
└── src/ICounter.vyi


"#
    ]]);
});

forgetest!(can_list_resolved_multiple_compiler_versions_skipped_json, |prj, cmd| {
    prj.add_source("ContractA", CONTRACT_A).unwrap();
    prj.add_source("ContractB", CONTRACT_B).unwrap();
    prj.add_source("ContractC", CONTRACT_C).unwrap();
    prj.add_source("ContractD", CONTRACT_D).unwrap();
    prj.add_raw_source("ICounter.vyi", VYPER_INTERFACE).unwrap();
    prj.add_raw_source("Counter.vy", VYPER_CONTRACT).unwrap();

    cmd.args(["compiler", "resolve", "--skip", "Contract(A|B|C)", "--json", "-v"])
        .assert_success()
        .stdout_eq(
            str![[r#"
{
  "Solidity": [
    {
      "name": "Solc",
      "version": "0.8.27",
      "paths": [
        "src/ContractD.sol"
      ]
    }
  ],
  "Vyper": [
    {
      "name": "Vyper",
      "version": "0.4.0",
      "paths": [
        "src/Counter.vy",
        "src/ICounter.vyi"
      ]
    }
  ]
}
"#]]
            .is_json(),
        );
});

forgetest!(can_list_resolved_multiple_compiler_versions_verbose, |prj, cmd| {
    prj.add_source("ContractA", CONTRACT_A).unwrap();
    prj.add_source("ContractB", CONTRACT_B).unwrap();
    prj.add_source("ContractC", CONTRACT_C).unwrap();
    prj.add_source("ContractD", CONTRACT_D).unwrap();
    prj.add_raw_source("ICounter.vyi", VYPER_INTERFACE).unwrap();
    prj.add_raw_source("Counter.vy", VYPER_CONTRACT).unwrap();

    cmd.args(["compiler", "resolve", "-vv"]).assert_success().stdout_eq(str![[r#"
Solidity:

Solc v0.8.4 (<= istanbul):
└── src/ContractA.sol

Solc v0.8.11 (<= london):
└── src/ContractB.sol

Solc v0.8.27 (<= cancun):
├── src/ContractC.sol
└── src/ContractD.sol

Vyper:

Vyper v0.4.0 (<= cancun):
├── src/Counter.vy
└── src/ICounter.vyi


"#]]);
});

forgetest!(can_list_resolved_multiple_compiler_versions_verbose_json, |prj, cmd| {
    prj.add_source("ContractA", CONTRACT_A).unwrap();
    prj.add_source("ContractB", CONTRACT_B).unwrap();
    prj.add_source("ContractC", CONTRACT_C).unwrap();
    prj.add_source("ContractD", CONTRACT_D).unwrap();
    prj.add_raw_source("ICounter.vyi", VYPER_INTERFACE).unwrap();
    prj.add_raw_source("Counter.vy", VYPER_CONTRACT).unwrap();

    cmd.args(["compiler", "resolve", "--json", "-vv"]).assert_success().stdout_eq(
        str![[r#"
{
  "Solidity": [
    {
      "name": "Solc",
      "version": "0.8.4",
      "evm_version": "Istanbul",
      "paths": [
        "src/ContractA.sol"
      ]
    },
    {
      "name": "Solc",
      "version": "0.8.11",
      "evm_version": "London",
      "paths": [
        "src/ContractB.sol"
      ]
    },
    {
      "name": "Solc",
      "version": "0.8.27",
      "evm_version": "[..]",
      "paths": [
        "src/ContractC.sol",
        "src/ContractD.sol"
      ]
    }
  ],
  "Vyper": [
    {
      "name": "Vyper",
      "version": "0.4.0",
      "evm_version": "[..]",
      "paths": [
        "src/Counter.vy",
        "src/ICounter.vyi"
      ]
    }
  ]
}
"#]]
        .is_json(),
    );
});
