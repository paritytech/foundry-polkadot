use crate::utils::{TestNode, unwrap_response};
use alloy_dyn_abi::TypedData;
use alloy_primitives::{Address, Bytes, Signature, U64};
use alloy_rpc_types::TransactionRequest;
use alloy_serde::WithOtherFields;
use anvil_core::eth::EthRequest;
use anvil_polkadot::{
    api_server::revive_conversions::{AlloyU256, ReviveAddress},
    config::{AnvilNodeConfig, SubstrateNodeConfig},
};
use polkadot_sdk::{
    pallet_revive::evm::{Account, TransactionSigned},
    sp_core::{H256, U256},
};
use std::time::Duration;

#[tokio::test(flavor = "multi_thread")]
async fn can_sign_transaction() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let alith = Account::from(subxt_signer::eth::dev::alith());
    let baltathar = Account::from(subxt_signer::eth::dev::baltathar());
    let alith_initial_balance = node.get_balance(alith.address(), None).await;
    let alith_addr = Address::from(ReviveAddress::new(alith.address()));
    let baltathar_addr = Address::from(ReviveAddress::new(baltathar.address()));
    let transfer_amount = alloy_primitives::U256::from_str_radix("100000000000000000", 10).unwrap();

    let gas_price: u128 =
        unwrap_response::<U256>(node.eth_rpc(EthRequest::EthGasPrice(())).await.unwrap())
            .unwrap()
            .try_into()
            .unwrap();
    let nonce: u64 = unwrap_response::<U256>(
        node.eth_rpc(EthRequest::EthGetTransactionCount(alith_addr, None)).await.unwrap(),
    )
    .unwrap()
    .try_into()
    .unwrap();
    let chain_id: u64 =
        unwrap_response::<U64>(node.eth_rpc(EthRequest::EthChainId(())).await.unwrap())
            .unwrap()
            .try_into()
            .unwrap();
    let mut transaction = TransactionRequest::default()
        .value(transfer_amount)
        .from(alith_addr)
        .to(baltathar_addr)
        .gas_price(gas_price)
        .nonce(nonce);
    let gas: u64 = unwrap_response::<U256>(
        node.eth_rpc(EthRequest::EthEstimateGas(
            WithOtherFields::new(transaction.clone()),
            None,
            None,
            None,
        ))
        .await
        .unwrap(),
    )
    .unwrap()
    .try_into()
    .unwrap();
    transaction.gas = Some(gas);
    transaction.chain_id = Some(chain_id);

    let signed_tx = unwrap_response::<TransactionSigned>(
        node.eth_rpc(EthRequest::EthSignTransaction(Box::new(WithOtherFields::new(transaction))))
            .await
            .unwrap(),
    )
    .unwrap()
    .signed_payload();

    let tx_hash = unwrap_response::<H256>(
        node.eth_rpc(EthRequest::EthSendRawTransaction(Bytes::from(signed_tx))).await.unwrap(),
    )
    .unwrap();
    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(None, None)).await.unwrap()).unwrap();
    tokio::time::sleep(Duration::from_millis(400)).await;

    let transaction_receipt = node.get_transaction_receipt(tx_hash).await;
    assert_eq!(transaction_receipt.from, alith.address());

    let alith_final_balance = node.get_balance(alith.address(), None).await;
    assert_eq!(
        alith_final_balance,
        alith_initial_balance
            - transfer_amount
            - AlloyU256::from(
                transaction_receipt.effective_gas_price * transaction_receipt.gas_used
            )
            .inner(),
        "Alith's balance should have changed"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn can_sign_typed_data() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();

    let json = serde_json::json!(
            {
      "types": {
        "EIP712Domain": [
          {
            "name": "name",
            "type": "string"
          },
          {
            "name": "version",
            "type": "string"
          },
          {
            "name": "chainId",
            "type": "uint256"
          },
          {
            "name": "verifyingContract",
            "type": "address"
          }
        ],
        "Person": [
          {
            "name": "name",
            "type": "string"
          },
          {
            "name": "wallet",
            "type": "address"
          }
        ],
        "Mail": [
          {
            "name": "from",
            "type": "Person"
          },
          {
            "name": "to",
            "type": "Person"
          },
          {
            "name": "contents",
            "type": "string"
          }
        ]
      },
      "primaryType": "Mail",
      "domain": {
        "name": "Ether Mail",
        "version": "1",
        "chainId": 1,
        "verifyingContract": "0xCcCCccccCCCCcCCCCCCcCcCccCcCCCcCcccccccC"
      },
      "message": {
        "from": {
          "name": "Cow",
          "wallet": "0xCD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826"
        },
        "to": {
          "name": "Bob",
          "wallet": "0xbBbBBBBbbBBBbbbBbbBbbbbBBbBbbbbBbBbbBBbB"
        },
        "contents": "Hello, Bob!"
      }
    });

    let signing_address: Address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".parse().unwrap();
    let typed_data: TypedData = serde_json::from_value(json).unwrap();
    let signature_hex = unwrap_response::<String>(
        node.eth_rpc(EthRequest::EthSignTypedDataV4(signing_address, typed_data.clone()))
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        signature_hex,
        "0x6ea8bb309a3401225701f3565e32519f94a0ea91a5910ce9229fe488e773584c0390416a2190d9560219dab757ecca2029e63fa9d1c2aebf676cc25b9f03126a1b"
    );
    let signature: Signature = signature_hex.parse().unwrap();
    let signed_hash = typed_data.eip712_signing_hash().unwrap();
    let recovered_address = signature.recover_address_from_prehash(&signed_hash).unwrap();
    assert_eq!(recovered_address, signing_address);
}

#[tokio::test(flavor = "multi_thread")]
async fn can_sign_typed_data_os() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();
    let json = serde_json::json!(
    {
      "types": {
        "EIP712Domain": [
          {
            "name": "name",
            "type": "string"
          },
          {
            "name": "version",
            "type": "string"
          },
          {
            "name": "chainId",
            "type": "uint256"
          },
          {
            "name": "verifyingContract",
            "type": "address"
          }
        ],
        "OrderComponents": [
          {
            "name": "offerer",
            "type": "address"
          },
          {
            "name": "zone",
            "type": "address"
          },
          {
            "name": "offer",
            "type": "OfferItem[]"
          },
          {
            "name": "consideration",
            "type": "ConsiderationItem[]"
          },
          {
            "name": "orderType",
            "type": "uint8"
          },
          {
            "name": "startTime",
            "type": "uint256"
          },
          {
            "name": "endTime",
            "type": "uint256"
          },
          {
            "name": "zoneHash",
            "type": "bytes32"
          },
          {
            "name": "salt",
            "type": "uint256"
          },
          {
            "name": "conduitKey",
            "type": "bytes32"
          },
          {
            "name": "counter",
            "type": "uint256"
          }
        ],
        "OfferItem": [
          {
            "name": "itemType",
            "type": "uint8"
          },
          {
            "name": "token",
            "type": "address"
          },
          {
            "name": "identifierOrCriteria",
            "type": "uint256"
          },
          {
            "name": "startAmount",
            "type": "uint256"
          },
          {
            "name": "endAmount",
            "type": "uint256"
          }
        ],
        "ConsiderationItem": [
          {
            "name": "itemType",
            "type": "uint8"
          },
          {
            "name": "token",
            "type": "address"
          },
          {
            "name": "identifierOrCriteria",
            "type": "uint256"
          },
          {
            "name": "startAmount",
            "type": "uint256"
          },
          {
            "name": "endAmount",
            "type": "uint256"
          },
          {
            "name": "recipient",
            "type": "address"
          }
        ]
      },
      "primaryType": "OrderComponents",
      "domain": {
        "name": "Seaport",
        "version": "1.1",
        "chainId": "1",
        "verifyingContract": "0x00000000006c3852cbEf3e08E8dF289169EdE581"
      },
      "message": {
        "offerer": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
        "offer": [
          {
            "itemType": "3",
            "token": "0xA604060890923Ff400e8c6f5290461A83AEDACec",
            "identifierOrCriteria": "110194434039389003190498847789203126033799499726478230611233094448886344768909",
            "startAmount": "1",
            "endAmount": "1"
          }
        ],
        "consideration": [
          {
            "itemType": "0",
            "token": "0x0000000000000000000000000000000000000000",
            "identifierOrCriteria": "0",
            "startAmount": "487500000000000000",
            "endAmount": "487500000000000000",
            "recipient": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
          },
          {
            "itemType": "0",
            "token": "0x0000000000000000000000000000000000000000",
            "identifierOrCriteria": "0",
            "startAmount": "12500000000000000",
            "endAmount": "12500000000000000",
            "recipient": "0x8De9C5A032463C561423387a9648c5C7BCC5BC90"
          }
        ],
        "startTime": "1658645591",
        "endTime": "1659250386",
        "orderType": "3",
        "zone": "0x004C00500000aD104D7DBd00e3ae0A5C00560C00",
        "zoneHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
        "salt": "16178208897136618",
        "conduitKey": "0x0000007b02230091a7ed01230072f7006a004d60a8d4e71d599b8104250f0000",
        "totalOriginalConsiderationItems": "2",
        "counter": "0"
      }
    }
        );

    let signing_address: Address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".parse().unwrap();
    let typed_data: TypedData = serde_json::from_value(json).unwrap();
    let signature_hex = unwrap_response::<String>(
        node.eth_rpc(EthRequest::EthSignTypedDataV4(signing_address, typed_data.clone()))
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        signature_hex,
        "0xedb0fa55ac67e3ca52b6bd6ee3576b193731adc2aff42151f67826932fa9f6191261ebdecc2c650204ff7625752b033293fb67ef5cfca78e16de359200040b761b"
    );
    let signature: Signature = signature_hex.parse().unwrap();
    let signed_hash = typed_data.eip712_signing_hash().unwrap();
    let recovered_address = signature.recover_address_from_prehash(&signed_hash).unwrap();
    assert_eq!(recovered_address, signing_address);
}

#[tokio::test(flavor = "multi_thread")]
async fn can_sign_typed_seaport_data() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config.clone(), substrate_node_config).await.unwrap();
    let json = serde_json::json!(
       {
      "types": {
        "EIP712Domain": [
          {
            "name": "name",
            "type": "string"
          },
          {
            "name": "version",
            "type": "string"
          },
          {
            "name": "chainId",
            "type": "uint256"
          },
          {
            "name": "verifyingContract",
            "type": "address"
          }
        ],
        "OrderComponents": [
          {
            "name": "offerer",
            "type": "address"
          },
          {
            "name": "zone",
            "type": "address"
          },
          {
            "name": "offer",
            "type": "OfferItem[]"
          },
          {
            "name": "consideration",
            "type": "ConsiderationItem[]"
          },
          {
            "name": "orderType",
            "type": "uint8"
          },
          {
            "name": "startTime",
            "type": "uint256"
          },
          {
            "name": "endTime",
            "type": "uint256"
          },
          {
            "name": "zoneHash",
            "type": "bytes32"
          },
          {
            "name": "salt",
            "type": "uint256"
          },
          {
            "name": "conduitKey",
            "type": "bytes32"
          },
          {
            "name": "counter",
            "type": "uint256"
          }
        ],
        "OfferItem": [
          {
            "name": "itemType",
            "type": "uint8"
          },
          {
            "name": "token",
            "type": "address"
          },
          {
            "name": "identifierOrCriteria",
            "type": "uint256"
          },
          {
            "name": "startAmount",
            "type": "uint256"
          },
          {
            "name": "endAmount",
            "type": "uint256"
          }
        ],
        "ConsiderationItem": [
          {
            "name": "itemType",
            "type": "uint8"
          },
          {
            "name": "token",
            "type": "address"
          },
          {
            "name": "identifierOrCriteria",
            "type": "uint256"
          },
          {
            "name": "startAmount",
            "type": "uint256"
          },
          {
            "name": "endAmount",
            "type": "uint256"
          },
          {
            "name": "recipient",
            "type": "address"
          }
        ]
      },
      "primaryType": "OrderComponents",
      "domain": {
        "name": "Seaport",
        "version": "1.1",
        "chainId": "137",
        "verifyingContract": "0x00000000006c3852cbEf3e08E8dF289169EdE581"
      },
      "message": {
        "offerer": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
        "offer": [
          {
            "itemType": "3",
            "token": "0xA604060890923Ff400e8c6f5290461A83AEDACec",
            "identifierOrCriteria": "110194434039389003190498847789203126033799499726478230611233094448886344768909",
            "startAmount": "1",
            "endAmount": "1"
          }
        ],
        "consideration": [
          {
            "itemType": "0",
            "token": "0x0000000000000000000000000000000000000000",
            "identifierOrCriteria": "0",
            "startAmount": "487500000000000000",
            "endAmount": "487500000000000000",
            "recipient": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
          },
          {
            "itemType": "0",
            "token": "0x0000000000000000000000000000000000000000",
            "identifierOrCriteria": "0",
            "startAmount": "12500000000000000",
            "endAmount": "12500000000000000",
            "recipient": "0x8De9C5A032463C561423387a9648c5C7BCC5BC90"
          }
        ],
        "startTime": "1658645591",
        "endTime": "1659250386",
        "orderType": "3",
        "zone": "0x004C00500000aD104D7DBd00e3ae0A5C00560C00",
        "zoneHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
        "salt": "16178208897136618",
        "conduitKey": "0x0000007b02230091a7ed01230072f7006a004d60a8d4e71d599b8104250f0000",
        "totalOriginalConsiderationItems": "2",
        "counter": "0"
      }
    }
            );

    let signing_address: Address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".parse().unwrap();
    let typed_data: TypedData = serde_json::from_value(json).unwrap();
    let signature_hex = unwrap_response::<String>(
        node.eth_rpc(EthRequest::EthSignTypedDataV4(signing_address, typed_data.clone()))
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        signature_hex,
        "0xed9afe7f377155ee3a42b25b696d79b55d441aeac7790b97a51b54ad0569b9665ea30bf8e8df12d6ee801c4dcb85ecfb8b23a6f7ae166d5af9acac9befb905451c"
    );
    let signature: Signature = signature_hex.parse().unwrap();
    let signed_hash = typed_data.eip712_signing_hash().unwrap();
    let recovered_address = signature.recover_address_from_prehash(&signed_hash).unwrap();
    assert_eq!(recovered_address, signing_address);
}
