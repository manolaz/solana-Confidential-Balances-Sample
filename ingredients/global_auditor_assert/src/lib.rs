use std::{error::Error, str::FromStr};

use keypair_utils::{get_rpc_client, load_value};
use solana_client::rpc_config::RpcTransactionConfig;
use solana_transaction_status::{
    self, parse_token, UiParsedInstruction, UiPartiallyDecodedInstruction,
};
use solana_transaction_status_client_types::{
    EncodedTransaction, UiInstruction, UiMessage, UiTransactionEncoding,
};
use spl_token_2022::solana_zk_sdk::encryption::elgamal::ElGamalKeypair;
// use solana_rpc_client_api::{
//     client_error::Error,
//     config::RpcTransactionConfig,
// };
// use solana_rpc_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{self, CompiledInstruction},
    pubkey::Pubkey,
    signature::{Signature, Signer},
    signer::keypair::Keypair,
    system_transaction,
};

use solana_sdk::message::AccountKeys;
// use solana_transaction_status_client_types::UiTransactionEncoding;

pub async fn last_transfer_amount(
    asserting_amount: u64,
    auditor_keypair: &ElGamalKeypair,
) -> Result<(), Box<dyn Error>> {
    let loaded_signature: String = load_value("last_confidential_transfer_signature")?;

    println!("Loaded signature: {:?}\n", loaded_signature);
    let signature = Signature::from_str(loaded_signature.as_str())?;

    let client = get_rpc_client()?;
    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::Json),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    };
    let tx = client.get_transaction_with_config(&signature, config)?;

    //println!("Tx: {:?}\n", tx);

    // Extract the transaction's message
    match tx.transaction.transaction {
        EncodedTransaction::Json(ui_transaction) => {
            if let UiMessage::Raw(raw_message) = ui_transaction.message {
                //let data = raw_message.instructions[0].data.clone();

                let mut prefixed_data = vec![27u8]; // 27 is the instruction type for confidential transfer
                prefixed_data.extend_from_slice(&raw_message.instructions[0].data.as_bytes());


                //println!("String Data: {:?}\n", data);
                //println!("Hex Data: {:?}\n", hex::encode(data.clone()));

                let compiled_instruction = CompiledInstruction {
                    program_id_index: raw_message.instructions[0].program_id_index,
                    accounts: raw_message.instructions[0].accounts.clone(),
                    
                    
                    data: prefixed_data,// THIS IS WRONG???
                };

                let keys_vec = raw_message
                    .account_keys
                    .iter()
                    .map(|key| Pubkey::from_str(key).unwrap())
                    .collect::<Vec<Pubkey>>();

                let account_keys =
                    solana_program::message::AccountKeys::new(keys_vec.as_slice(), None);
                

                let parsed_token = parse_token::parse_token(&compiled_instruction, &account_keys)?;
                println!("Parsed token: {:?}\n", parsed_token.instruction_type);
            }
        }
        _ => println!("Unexpected transaction encoding"),
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_last_transfer_amount() -> Result<(), Box<dyn Error>> {
        last_transfer_amount(100, &ElGamalKeypair::new_rand()).await?;
        Ok(())
    }
}
