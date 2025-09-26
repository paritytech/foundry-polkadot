// Copyright (C) 2023 Polytope Labs (Caymans) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Host function overrides for signature verification.

use polkadot_sdk::sp_io::{self, EcdsaVerifyError};
use sp_runtime_interface::{
    pass_by::{
        AllocateAndReturnByCodec, AllocateAndReturnPointer, PassFatPointerAndRead,
        PassPointerAndRead,
    },
    runtime_interface,
};

#[runtime_interface]
pub trait Crypto {
    #[version(1)]
    fn secp256k1_ecdsa_recover(
        sig: PassPointerAndRead<&[u8; 65], 65>,
        msg: PassPointerAndRead<&[u8; 32], 32>,
    ) -> AllocateAndReturnByCodec<Result<[u8; 64], EcdsaVerifyError>> {
        if sig[..12].iter().eq([0; 12].iter()) {
            trace!(
                target = "host_fn_overrides",
                name = "secp256k1_ecdsa_recover - version 1",
                "impersonation for: {:?}",
                &sig[12..32]
            );
            let mut res = [0u8; 64];
            res[12..32].copy_from_slice(&sig[12..32]);
            Ok(res)
        } else {
            sp_io::crypto::secp256k1_ecdsa_recover(sig, msg)
        }
    }

    #[version(2)]
    fn secp256k1_ecdsa_recover(
        sig: PassPointerAndRead<&[u8; 65], 65>,
        msg: PassPointerAndRead<&[u8; 32], 32>,
    ) -> AllocateAndReturnByCodec<Result<[u8; 64], EcdsaVerifyError>> {
        if sig[..12] == [0; 12] && sig[32..64] == [0; 32] {
            trace!(
                target = "host_fn_overrides",
                name = "secp256k1_ecdsa_recover - version 2",
                "impersonation for: {:?}",
                &sig[12..32]
            );
            let mut res = [0u8; 64];
            res[12..32].copy_from_slice(&sig[12..32]);
            Ok(res)
        } else {
            sp_io::crypto::secp256k1_ecdsa_recover(sig, msg)
        }
    }
}

#[runtime_interface]
pub trait Hashing {
    fn keccak_256(data: PassFatPointerAndRead<&[u8]>) -> AllocateAndReturnPointer<[u8; 32], 32> {
        if data.len() == 64 && data[..12] == [0; 12] && data[32..64] == [0; 32] {
            trace!(
                target = "host_fn_overrides",
                name = "keccak_256",
                "impersonation for: {:?}",
                &data[12..32]
            );
            let mut res = [0; 32];
            res.copy_from_slice(&data[0..32]);
            res
        } else {
            sp_io::hashing::keccak_256(data)
        }
    }
}

/// Provides host function that overrides ETH address recovery from
/// signature in the scope of impersonation.
pub type SenderAddressRecoveryOverride = self::crypto::HostFunctions;
/// Provides host function that overrided hashing functions in the
/// scope of impersonation.
pub type PublicKeyToHashOverride = self::hashing::HostFunctions;
