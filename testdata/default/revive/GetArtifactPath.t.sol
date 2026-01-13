// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity =0.8.18;

import "ds-test/test.sol";
import "cheats/Vm.sol";

contract DummyForGetArtifactPath {}

contract GetArtifactPathTest is DSTest {
    Vm constant vm = Vm(HEVM_ADDRESS);

    function testGetArtifactPathByCode() public {
        bytes memory dummyCreationCode = type(DummyForGetArtifactPath).creationCode;

        string memory path = vm.getArtifactPathByCode(dummyCreationCode);
        assertTrue(vm.contains(path, "/resolc-out/default/solc/revive/GetArtifactPath.t.sol/DummyForGetArtifactPath.json"));
    }

    function testGetArtifactPathByDeployedCode() public {
        bytes memory dummyRuntimeCode = vm.getDeployedCode("revive/GetArtifactPath.t.sol:DummyForGetArtifactPath:0.8.18");

        string memory path = vm.getArtifactPathByDeployedCode(dummyRuntimeCode);
        assertTrue(vm.contains(path, "/resolc-out/default/solc/revive/GetArtifactPath.t.sol/DummyForGetArtifactPath.json"));
    }
}
