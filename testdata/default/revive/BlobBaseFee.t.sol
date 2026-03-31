// // SPDX-License-Identifier: MIT OR Apache-2.0
// pragma solidity ^0.8.25;

// import "utils/DSTest.sol";
// import "utils/Vm.sol";

// contract BlockBlobBaseFee {
//     function blobBaseFee() public view returns (uint256) {
//         uint256 fee;
//         assembly {
//             fee := blobbasefee()
//         }
//         return fee;
//     }
// }

// contract BlobBaseFeeTest is DSTest {
//     Vm constant vm = Vm(HEVM_ADDRESS);

//     function testBlobBaseFee() public {
//         BlockBlobBaseFee blobContract = new BlockBlobBaseFee();
//         vm.blobBaseFee(6969);
//         assertEq(vm.getBlobBaseFee(), 6969);
//         assertEq(blobContract.blobBaseFee(), 6969, "blobbasefee failed");
//     }
// }
