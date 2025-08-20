// TODO: False positive, after switch to PVM we still read balance from EVM
forgetest!(can_translate_balances_after_switch_to_pvm, |prj, cmd| {
    prj.insert_ds_test();
    prj.insert_vm();

    prj.add_source(
        "BalanceTranslationTest.t.sol",
        r#"
import "./test.sol";
import "./Vm.sol";

contract BalanceTranslationTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function test_BalanceTranslationRevmPvm() public {
        uint256 amount = 100 ether;
        vm.deal(address(this), amount);
        uint256 initialBalance = address(this).balance;
        assertEq(initialBalance, amount);

        vm.pvm(true);

        uint256 currentBalance = address(this).balance;
        assertEq(initialBalance, currentBalance);
    }
}
"#,
    )
    .unwrap();

    cmd.args(["test"]).assert_success();
});
