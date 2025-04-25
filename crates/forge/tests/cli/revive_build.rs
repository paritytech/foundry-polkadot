use foundry_test_utils::snapbox::IntoData;

use crate::utils::generate_large_init_contract;

forgetest_init!(can_build_with_resolc, |prj, cmd| {
    cmd.args(["build", "--resolc-compile"]).assert_success();
});

forgetest_init!(force_buid_with_resolc, |prj, cmd| {
    cmd.args(["build", "--resolc-compile", "--force"]).assert_success();
});

forgetest!(code_size_exceeds_limit_with_resolc, |prj, cmd| {
    prj.add_source("LargeContract.sol", generate_large_init_contract(50_000).as_str()).unwrap();
    cmd.args(["build", "--resolc-compile", "--sizes"]).assert_failure().stdout_eq(str![[r#"
[COMPILING_FILES] with [RESOLC_VERSION]
[RESOLC_VERSION] [ELAPSED]
Compiler run successful!

╭---------------+------------------+-------------------+--------------------+---------------------╮
| Contract      | Runtime Size (B) | Initcode Size (B) | Runtime Margin (B) | Initcode Margin (B) |
+=================================================================================================+
| LargeContract | 264,700          | 264,700           | -14,700            | -14,700             |
╰---------------+------------------+-------------------+--------------------+---------------------╯


"#]]);

    cmd.forge_fuse()
        .args(["build", "--resolc-compile", "--sizes", "--json"])
        .assert_failure()
        .stdout_eq(
            str![[r#"
{
  "LargeContract": {
    "runtime_size": 264700,
    "init_size": 264700,
    "runtime_margin": -14700,
    "init_margin": -14700
  }
}
"#]]
            .is_json(),
        );
});
