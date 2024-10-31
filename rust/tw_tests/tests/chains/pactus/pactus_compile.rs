// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::chains::pactus::test_cases::PRIVATE_KEY;
use crate::chains::pactus::test_cases::{bond_with_public_key_test_case, bond_without_public_key_test_case, transfer_test_case};
use tw_any_coin::ffi::tw_transaction_compiler::{
    tw_transaction_compiler_compile, tw_transaction_compiler_pre_image_hashes,
};
use tw_coin_entry::error::prelude::*;
use tw_coin_registry::coin_type::CoinType;
use tw_encoding::hex::ToHex;
use tw_keypair::ed25519;
use tw_keypair::traits::{KeyPairTrait, SigningKeyTrait};
use tw_memory::test_utils::tw_data_helper::TWDataHelper;
use tw_memory::test_utils::tw_data_vector_helper::TWDataVectorHelper;
use tw_misc::traits::ToBytesVec;
use tw_proto::Pactus::Proto;
use tw_proto::TxCompiler::Proto as CompilerProto;
use tw_proto::{deserialize, serialize};

struct TestCase {
    sign_input_fn: fn() -> Proto::SigningInput<'static>,
    expected_sign_bytes: &'static str,
    expected_signature: &'static str,
    expected_tx_id: &'static str,
    expected_signed_data: &'static str,
}

#[test]
fn test_pactus_transaction_compile() {
    let cases = [
        TestCase {
            sign_input_fn: transfer_test_case::sign_input,
            expected_sign_bytes: transfer_test_case::SIGN_BYTES,
            expected_signature: transfer_test_case::SIGNATURE,
            expected_tx_id: transfer_test_case::TX_ID,
            expected_signed_data: transfer_test_case::SIGNED_DATA,
        },
        TestCase {
            sign_input_fn: bond_with_public_key_test_case::sign_input,
            expected_sign_bytes: bond_with_public_key_test_case::SIGN_BYTES,
            expected_signature: bond_with_public_key_test_case::SIGNATURE,
            expected_tx_id: bond_with_public_key_test_case::TX_ID,
            expected_signed_data: bond_with_public_key_test_case::SIGNED_DATA,
        },
        TestCase {
            sign_input_fn: bond_without_public_key_test_case::sign_input,
            expected_sign_bytes: bond_without_public_key_test_case::SIGN_BYTES,
            expected_signature: bond_without_public_key_test_case::SIGNATURE,
            expected_tx_id: bond_without_public_key_test_case::TX_ID,
            expected_signed_data: bond_without_public_key_test_case::SIGNED_DATA,
        },
    ];

    for case in cases.iter() {
        // Step 1: Create signing input.
        let input = (case.sign_input_fn)();

        // Step 2: Obtain preimage hash
        let input_data = TWDataHelper::create(serialize(&input).unwrap());
        let preimage_data = TWDataHelper::wrap(unsafe {
            tw_transaction_compiler_pre_image_hashes(CoinType::Pactus as u32, input_data.ptr())
        })
        .to_vec()
        .expect("!tw_transaction_compiler_pre_image_hashes returned nullptr");

        let preimage: CompilerProto::PreSigningOutput =
            deserialize(&preimage_data).expect("Coin entry returned an invalid output");

        assert_eq!(preimage.error, SigningErrorType::OK);
        assert!(preimage.error_message.is_empty());
        assert_eq!(preimage.data.to_hex(), case.expected_sign_bytes);

        // Step 3: Sign the data "externally"
        let private_key = ed25519::sha512::KeyPair::try_from(PRIVATE_KEY).unwrap();
        let public_key = private_key.public().to_vec();

        let signature = private_key
            .sign(preimage.data.to_vec())
            .expect("Error signing data")
            .to_vec();
        assert_eq!(signature.to_hex(), case.expected_signature);

        // Step 4: Compile transaction info
        let signatures = TWDataVectorHelper::create([signature]);
        let public_keys = TWDataVectorHelper::create([public_key]);

        let input_data = TWDataHelper::create(serialize(&input).unwrap());
        let output_data = TWDataHelper::wrap(unsafe {
            tw_transaction_compiler_compile(
                CoinType::Pactus as u32,
                input_data.ptr(),
                signatures.ptr(),
                public_keys.ptr(),
            )
        })
        .to_vec()
        .expect("!tw_transaction_compiler_compile returned nullptr");

        let output: Proto::SigningOutput =
            deserialize(&output_data).expect("Coin entry returned an invalid output");

        assert_eq!(output.error, SigningErrorType::OK);
        assert!(output.error_message.is_empty());
        assert_eq!(output.transaction_id.to_hex(), case.expected_tx_id);
        assert_eq!(output.signature.to_hex(), case.expected_signature);
        assert_eq!(
            output.signed_transaction_data.to_hex(),
            case.expected_signed_data
        );
    }
}
