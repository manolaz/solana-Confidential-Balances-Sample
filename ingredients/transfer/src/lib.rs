use std::time::Duration;

use serde_json::json;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_zk_sdk::zk_elgamal_proof_program;

use {
    jito_sdk_rust::JitoJsonRpcSDK, keypair_utils::{get_non_blocking_rpc_client, get_or_create_keypair, get_rpc_client, load_value, record_value}, solana_sdk::{
        pubkey::Pubkey, signature::{Keypair, Signer}, system_instruction, transaction::Transaction
    }, spl_associated_token_account::
        get_associated_token_address_with_program_id, spl_token_2022::{
        extension::{
            confidential_transfer::{
                account_info::
                    TransferAccountInfo
                ,
                ConfidentialTransferAccount, ConfidentialTransferMint,
            },
            BaseStateWithExtensions, StateWithExtensionsOwned,
        },
        solana_zk_sdk::{
            encryption::{
                auth_encryption::AeKey,
                elgamal::{self, ElGamalKeypair},
                pod::elgamal::PodElGamalPubkey,
            },
            zk_elgamal_proof_program::instruction::{close_context_state, ContextStateInfo},
        },
        state::{Account, Mint},
    }, spl_token_client::{
        client::{ProgramRpcClient, ProgramRpcClientSendTransaction, RpcClientResponse},
        token::{ProofAccount, ProofAccountWithCiphertext, Token},
    }, spl_token_confidential_transfer_proof_generation::
        transfer::TransferProofData, std::{error::Error, str::FromStr, sync::Arc}
};

use reqwest;
use serde_json::Value;
async fn get_max_jito_tip_amount() -> Result<u64, Box<dyn std::error::Error>> {
    // Query the API
    let response = reqwest::get("https://bundles.jito.wtf/api/v1/bundles/tip_floor").await?;
    let data: Value = response.json().await?;

    // Parse the JSON to get the 99th percentile tip
    let landed_tips_99th_percentile = data[0]["landed_tips_99th_percentile"]
        .as_f64()
        .ok_or("Failed to parse landed_tips_99th_percentile")?;

    println!("Jito landed_tips_99th_percentile: {}", landed_tips_99th_percentile);

    // Convert SOL to Lamports
    let jito_tip_amount = (landed_tips_99th_percentile * LAMPORTS_PER_SOL as f64) as u64;

    Ok(jito_tip_amount)
}

pub async fn with_split_proofs(sender_keypair: Arc<dyn Signer>, recipient_keypair: Arc<dyn Signer>, confidential_transfer_amount: u64) -> Result<(), Box<dyn Error>> {   

    let client = get_rpc_client()?;
    let transactions = prepare_transactions(sender_keypair.clone(), recipient_keypair, confidential_transfer_amount).await?;
    assert!(transactions.len() == 5);

    println!(
        "\nTransfer [Allocate Proof Accounts]: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899",
        client.send_and_confirm_transaction(&transactions[0])?
    );
    println!(
        "\nTransfer [Encode Range Proof]: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899",
        client.send_and_confirm_transaction(&transactions[1])?
    );
    println!(
        "\nTransfer [Encode Remaining Proofs]: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899",
        client.send_and_confirm_transaction(&transactions[2])?
    );

    let signature = client.send_and_confirm_transaction(&transactions[3])?.to_string();
    record_value("last_confidential_transfer_signature", &signature)?;
    println!(
        "\nTransfer [Execute Transfer]: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899",
        signature
    );
    
    println!(
        "\nTransfer [Close Proof Accounts]: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899",
        client.send_and_confirm_transaction(&transactions[4])?
    );

    Ok(())

}

async fn prepare_transactions(sender_keypair: Arc<dyn Signer>, recipient_keypair: Arc<dyn Signer>, confidential_transfer_amount: u64) -> Result<Vec<Transaction>, Box<dyn Error>> {
    let client = get_rpc_client()?;

    let mint = get_or_create_keypair("mint")?;
    let sender_associated_token_address: Pubkey = get_associated_token_address_with_program_id(
        &sender_keypair.pubkey(),
        &mint.pubkey(),
        &spl_token_2022::id(),
    );
    let decimals = load_value("mint_decimals")?;

    let token = {
        let rpc_client = get_non_blocking_rpc_client()?;

        let program_client: ProgramRpcClient<ProgramRpcClientSendTransaction> =
            ProgramRpcClient::new(Arc::new(rpc_client), ProgramRpcClientSendTransaction);

        // Create a "token" client, to use various helper functions for Token Extensions
        Token::new(
            Arc::new(program_client),
            &spl_token_2022::id(),
            &mint.pubkey(),
            Some(decimals),
            sender_keypair.clone(),
            /*
            Can't use the intended separate fee_payer_keypair because I get the following error:
            Client(Error { 
                request: Some(SendTransaction), 
                kind: RpcError(RpcResponseError { 
                    code: -32602, 
                    message: "base64 encoded solana_sdk::transaction::versioned::VersionedTransaction too large: 1652 bytes (max: encoded/raw 1644/1232)", 
                    data: Empty 
                }) 
            })

            Makes me wonder if transaction has one too many signers for range proof.
            */
        )
    };
    let recipient_associated_token_address = get_associated_token_address_with_program_id(
        &recipient_keypair.pubkey(),
        &mint.pubkey(),
        &spl_token_2022::id(),
    );

    // Must first create 3 accounts to store proofs before transferring tokens
    // This must be done in a separate transactions because the proofs are too large for single transaction

    // Equality Proof - prove that two ciphertexts encrypt the same value
    // Ciphertext Validity Proof - prove that ciphertexts are properly generated
    // Range Proof - prove that ciphertexts encrypt a value in a specified range (0, u64::MAX)

    // "Authority" for the proof accounts (to close the accounts after the transfer)
    let context_state_authority = &sender_keypair;

    // Generate address for equality proof account
    let equality_proof_context_state_account = Keypair::new();
    let equality_proof_pubkey = equality_proof_context_state_account.pubkey();

    // Generate address for ciphertext validity proof account
    let ciphertext_validity_proof_context_state_account = Keypair::new();
    let ciphertext_validity_proof_pubkey = ciphertext_validity_proof_context_state_account.pubkey();

    // Generate address for range proof account
    let range_proof_context_state_account = Keypair::new();
    let range_proof_pubkey = range_proof_context_state_account.pubkey();

    // Get sender token account data
    let sender_token_account_info = token
        .get_account_info(&sender_associated_token_address)
        .await?;

    let sender_account_extension_data =
        sender_token_account_info.get_extension::<ConfidentialTransferAccount>()?;

    // ConfidentialTransferAccount extension information needed to create proof data
    let sender_transfer_account_info = TransferAccountInfo::new(sender_account_extension_data);

    let sender_elgamal_keypair =
        ElGamalKeypair::new_from_signer(&sender_keypair, &sender_associated_token_address.to_bytes())?;
    let sender_aes_key =
        AeKey::new_from_signer(&sender_keypair, &sender_associated_token_address.to_bytes())?;

    // Get recipient token account data
    let recipient_account = token
        .get_account(recipient_associated_token_address)
        .await?;

    // Get recipient ElGamal pubkey from the recipient token account data and convert to elgamal::ElGamalPubkey
    let recipient_elgamal_pubkey: elgamal::ElGamalPubkey =
        StateWithExtensionsOwned::<Account>::unpack(recipient_account.data)?
            .get_extension::<ConfidentialTransferAccount>()?
            .elgamal_pubkey
            .try_into()?;

    // Get mint account data
    let mint_account = token.get_account(mint.pubkey()).await?;

    // Get auditor ElGamal pubkey from the mint account data
    let auditor_elgamal_pubkey_option = Option::<PodElGamalPubkey>::from(
        StateWithExtensionsOwned::<Mint>::unpack(mint_account.data)?
            .get_extension::<ConfidentialTransferMint>()?
            .auditor_elgamal_pubkey,
    );

    // Convert auditor ElGamal pubkey to elgamal::ElGamalPubkey type
    let auditor_elgamal_pubkey: elgamal::ElGamalPubkey = auditor_elgamal_pubkey_option
        .ok_or("No Auditor ElGamal pubkey")?
        .try_into()?;

    // Generate proof data
    let TransferProofData {
        equality_proof_data,
        ciphertext_validity_proof_data_with_ciphertext,
        range_proof_data,
    } = sender_transfer_account_info.generate_split_transfer_proof_data(
        confidential_transfer_amount,
        &sender_elgamal_keypair,
        &sender_aes_key,
        &recipient_elgamal_pubkey,
        Some(&auditor_elgamal_pubkey),
    )?;

    // Create 3 proofs ------------------------------------------------------

    // Range Proof Instructions------------------------------------------------------------------------------
    let (range_create_ix, range_verify_ix) = get_zk_proof_context_state_account_creation_instructions(
        &sender_keypair.pubkey(),
        &range_proof_context_state_account.pubkey(),
        &context_state_authority.pubkey(),
        &range_proof_data,
    )?;

    // Equality Proof Instructions---------------------------------------------------------------------------
    let (equality_create_ix, equality_verify_ix) = get_zk_proof_context_state_account_creation_instructions(
        &sender_keypair.pubkey(),
        &equality_proof_context_state_account.pubkey(),
        &context_state_authority.pubkey(),
        &equality_proof_data,
    )?;

    // Ciphertext Validity Proof Instructions ----------------------------------------------------------------
    let (cv_create_ix, cv_verify_ix) = get_zk_proof_context_state_account_creation_instructions(
        &sender_keypair.pubkey(),
        &ciphertext_validity_proof_context_state_account.pubkey(),
        &context_state_authority.pubkey(),
        &ciphertext_validity_proof_data_with_ciphertext.proof_data,
    )?;


    // Transact Proofs ------------------------------------------------------------------------------------
    
    // Transaction 1: Allocate all proof accounts at once.
    let tx1 = Transaction::new_signed_with_payer(
        &[range_create_ix.clone(), equality_create_ix.clone(), cv_create_ix.clone()],
        Some(&sender_keypair.pubkey()),
        &[
            &sender_keypair, 
            &range_proof_context_state_account as &dyn Signer, 
            &equality_proof_context_state_account as &dyn Signer, 
            &ciphertext_validity_proof_context_state_account as &dyn Signer
        ],
        client.get_latest_blockhash()?,
    );
    
    // Transaction 2: Encode Range Proof on its own (because it's the largest).
    let tx2 = Transaction::new_signed_with_payer(
        &[range_verify_ix],
        Some(&sender_keypair.pubkey()),
        &[&sender_keypair],
        client.get_latest_blockhash()?,
    );

    // Transaction 3: Encode all remaining proofs.
    let tx3 = Transaction::new_signed_with_payer(
        &[equality_verify_ix, cv_verify_ix],
        Some(&sender_keypair.pubkey()),
        &[&sender_keypair],
        client.get_latest_blockhash()?,
    );
    
    // Transaction 4: Execute transfer (below)
    // Transfer with Split Proofs -------------------------------------------

    let equality_proof_context_proof_account = ProofAccount::ContextAccount(equality_proof_pubkey);
    let ciphertext_validity_proof_context_proof_account =
        ProofAccount::ContextAccount(ciphertext_validity_proof_pubkey);
    let range_proof_context_proof_account = ProofAccount::ContextAccount(range_proof_pubkey);

    let ciphertext_validity_proof_account_with_ciphertext = ProofAccountWithCiphertext {
        proof_account: ciphertext_validity_proof_context_proof_account,
        ciphertext_lo: ciphertext_validity_proof_data_with_ciphertext.ciphertext_lo,
        ciphertext_hi: ciphertext_validity_proof_data_with_ciphertext.ciphertext_hi,
    };

    let tx4 = token.confidential_transfer_transfer_tx(
        &sender_associated_token_address,
        &recipient_associated_token_address,
        &sender_keypair.pubkey(),
        Some(&equality_proof_context_proof_account),
        Some(&ciphertext_validity_proof_account_with_ciphertext),
        Some(&range_proof_context_proof_account),
        confidential_transfer_amount,
        Some(sender_transfer_account_info),
        &sender_elgamal_keypair,
        &sender_aes_key,
        &recipient_elgamal_pubkey,
        Some(&auditor_elgamal_pubkey),
        &[&sender_keypair],
    ).await?;

    // Transaction 5: (below)
    // Close Proof Accounts --------------------------------------------------

    // Authority to close the proof accounts
    let context_state_authority_pubkey = context_state_authority.pubkey();
    // Lamports from the closed proof accounts will be sent to this account
    let destination_account = &sender_keypair.pubkey();

    // Close the equality proof account
    let close_equality_proof_instruction = close_context_state(
        ContextStateInfo {
            context_state_account: &equality_proof_pubkey,
            context_state_authority: &context_state_authority_pubkey,
        },
        &destination_account,
    );

    // Close the ciphertext validity proof account
    let close_ciphertext_validity_proof_instruction = close_context_state(
        ContextStateInfo {
            context_state_account: &ciphertext_validity_proof_pubkey,
            context_state_authority: &context_state_authority_pubkey,
        },
        &destination_account,
    );

    // Close the range proof account
    let close_range_proof_instruction = close_context_state(
        ContextStateInfo {
            context_state_account: &range_proof_pubkey,
            context_state_authority: &context_state_authority_pubkey,
        },
        &destination_account,
    );

    let recent_blockhash = client.get_latest_blockhash()?;
    let tx5 = Transaction::new_signed_with_payer(
        &[
            close_equality_proof_instruction,
            close_ciphertext_validity_proof_instruction,
            close_range_proof_instruction,
        ],
        Some(&sender_keypair.pubkey()),
        &[&sender_keypair], // Signers
        recent_blockhash,
    );

    Ok(vec![tx1, tx2, tx3, tx4, tx5])
}
pub async fn with_split_proofs_atomic(sender_keypair: Arc<dyn Signer>, recipient_keypair: Arc<dyn Signer>, confidential_transfer_amount: u64) -> Result<(), Box<dyn Error>> {
    
    let mut transactions = prepare_transactions(sender_keypair.clone(), recipient_keypair, confidential_transfer_amount).await?;
    let jito_sdk = JitoJsonRpcSDK::new("https://dallas.testnet.block-engine.jito.wtf/api/v1", None);

    // Reconstruct the one transaction to add the jito tip instruction.
    {
        let random_tip_account = jito_sdk.get_random_tip_account().await?;
        let jito_tip_account = Pubkey::from_str(&random_tip_account)?;
        let jito_tip_amount:u64 = get_max_jito_tip_amount().await?;
        println!("Jito tip lamports: {}", jito_tip_amount);
        let jito_tip_ix = system_instruction::transfer(
            &sender_keypair.pubkey(),
            &jito_tip_account,
            jito_tip_amount,
        );

        let client = get_rpc_client()?;
        assert!(client.url().contains("testnet") || client.url().contains("mainnet"), "This Jito demo only works on testnet or mainnet (adjust code for custom endpoints)");
        
        // Any transaction can be used. This one is the simplest to edit (and fits within size limits).
        let tx3 = &mut transactions[2];

        // Include instruction's accounts into the transaction (without duplicates).
        {
            let mut unique_pubkeys: std::collections::HashSet<_> = tx3.message.account_keys.iter().cloned().collect();
            tx3.message.account_keys.extend(
                jito_tip_ix
                    .accounts
                    .iter()
                    .map(|account| account.pubkey)
                    .filter(|pubkey| unique_pubkeys.insert(*pubkey))
            );

            tx3.message.account_keys.push(solana_sdk::system_program::id());
        }
        
        // Include instruction into the transaction.
        let compiled_jito_tip_ix = tx3.message.compile_instruction(&jito_tip_ix);
        tx3.message.instructions.push(compiled_jito_tip_ix);

        // Re-sign the transaction for integrity.
        tx3.sign(&[&sender_keypair], client.get_latest_blockhash()?);

    }

    let serialized_tx1 = bs58::encode(bincode::serialize(&transactions[0])?).into_string();
    let serialized_tx2 = bs58::encode(bincode::serialize(&transactions[1])?).into_string();
    let serialized_tx3 = bs58::encode(bincode::serialize(&transactions[2])?).into_string();
    let serialized_tx4 = bs58::encode(bincode::serialize(&transactions[3])?).into_string();
    let serialized_tx5 = bs58::encode(bincode::serialize(&transactions[4])?).into_string();
    
    let tx_bundle = json!([
        serialized_tx1, 
        serialized_tx2, 
        serialized_tx3, 
        serialized_tx4, 
        serialized_tx5
    ]);
    
    // UUID for the bundle
    let uuid = None;

    // Send bundle using Jito SDK
    println!("Sending bundle with {} transactions...", transactions.len());
    let response = jito_sdk.send_bundle(Some(tx_bundle), uuid).await?;

    // Extract bundle UUID from response
    let bundle_uuid = response["result"]
        .as_str()
        .ok_or("Failed to get bundle UUID from response")?;
    println!("Bundle sent with UUID: {}", bundle_uuid);

    let bundled_signatures = confirm_bundle_status(&jito_sdk, &bundle_uuid).await?;

    record_value("last_confidential_transfer_signature", &bundled_signatures[3])?;

    Ok(())
}
/// Refactored version of spl_token_client::token::Token::confidential_transfer_create_context_state_account().
/// Instead of sending transactions internally, this function now returns the instructions to be used externally.
fn get_zk_proof_context_state_account_creation_instructions<
    ZK: bytemuck::Pod + zk_elgamal_proof_program::proof_data::ZkProofData<U>,
    U: bytemuck::Pod,
>(
    fee_payer_pubkey: &Pubkey,
    context_state_account_pubkey: &Pubkey,
    context_state_authority_pubkey: &Pubkey,
    proof_data: &ZK,
) -> Result<(solana_sdk::instruction::Instruction, solana_sdk::instruction::Instruction), Box<dyn Error>> {
    use std::mem::size_of;
    use spl_token_confidential_transfer_proof_extraction::instruction::zk_proof_type_to_instruction;

    let client = get_rpc_client()?;
    let space = size_of::<zk_elgamal_proof_program::state::ProofContextState<U>>();
    let rent = client.get_minimum_balance_for_rent_exemption(space)?;

    let context_state_info = ContextStateInfo {
        context_state_account: context_state_account_pubkey,
        context_state_authority: context_state_authority_pubkey,
    };

    let instruction_type = zk_proof_type_to_instruction(ZK::PROOF_TYPE)?;

    let create_account_ix = system_instruction::create_account(
        fee_payer_pubkey,
        context_state_account_pubkey,
        rent,
        space as u64,
        &zk_elgamal_proof_program::id(),
    );

    let verify_proof_ix =
        instruction_type.encode_verify_proof(Some(context_state_info), proof_data);

    // Return a tuple containing the create account instruction and verify proof instruction.
    Ok((create_account_ix, verify_proof_ix))
}

#[derive(Debug)]
struct BundleStatus {
    confirmation_status: Option<String>,
    err: Option<serde_json::Value>,
    transactions: Option<Vec<String>>,
}

const MAX_RETRIES: u32 = 30;
const RETRY_DELAY: Duration = Duration::from_secs(3);
async fn confirm_bundle_status(jito_sdk: &JitoJsonRpcSDK, bundle_uuid: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {

    for attempt in 1..=MAX_RETRIES {
        println!("Checking bundle status (attempt {}/{})", attempt, MAX_RETRIES);

        let status_response = jito_sdk.get_in_flight_bundle_statuses(vec![bundle_uuid.to_string()]).await?;

        if let Some(result) = status_response.get("result") {
            if let Some(value) = result.get("value") {
                if let Some(statuses) = value.as_array() {
                    if let Some(bundle_status) = statuses.get(0) {
                        if let Some(status) = bundle_status.get("status") {
                            match status.as_str() {
                                Some("Landed") => {
                                    println!("Bundle landed on-chain. Checking final status...");
                                    return Ok(check_final_bundle_status(&jito_sdk, bundle_uuid).await?);
                                },
                                Some("Pending") => {
                                    println!("Bundle is pending. Waiting...");
                                },
                                Some(status) => {
                                    if status == "Failed" {
                                        return Err(format!("Bundle failed to land on-chain").into());
                                    }
                                    println!("Unexpected bundle status: {}. Waiting...", status);
                                },
                                None => {
                                    println!("Unable to parse bundle status. Waiting...");
                                }
                            }
                        } else {
                            println!("Status field not found in bundle status. Waiting...");
                        }
                    } else {
                        println!("Bundle status not found. Waiting...");
                    }
                } else {
                    println!("Unexpected value format. Waiting...");
                }
            } else {
                println!("Value field not found in result. Waiting...");

            }
        } else if let Some(error) = status_response.get("error") {
            println!("Error checking bundle status: {:?}", error);
        } else {
            println!("Unexpected response format. Waiting...");
        }

        if attempt < MAX_RETRIES {
            std::thread::sleep(RETRY_DELAY);
        }
    }

    Err(format!("Failed to confirm bundle status after {} attempts", MAX_RETRIES).into())
}

async fn check_final_bundle_status(jito_sdk: &JitoJsonRpcSDK, bundle_uuid: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {

    for attempt in 1..=MAX_RETRIES {
        println!("Checking final bundle status (attempt {}/{})", attempt, MAX_RETRIES);

        let status_response = jito_sdk.get_bundle_statuses(vec![bundle_uuid.to_string()]).await?;
        let bundle_status = get_bundle_status(&status_response)?;

        match bundle_status.confirmation_status.as_deref() {
            Some("confirmed") => {
                println!("Bundle confirmed on-chain. Waiting for finalization...");
                check_transaction_error(&bundle_status)?;
            },
            Some("finalized") => {
                println!("Bundle finalized on-chain successfully!");
                check_transaction_error(&bundle_status)?;
                print_transaction_url(&bundle_status);
                return match bundle_status.transactions {
                    Some(transactions) => Ok(transactions),
                    None => Err("Error retrieving transactions from finalized bundle status".into()),
                };
            },
            Some(status) => {
                println!("Unexpected final bundle status: {}. Continuing to poll...", status);
            },
            None => {
                println!("Unable to parse final bundle status. Continuing to poll...");
            }
        }

        if attempt < MAX_RETRIES {
            std::thread::sleep(RETRY_DELAY);
        }
    }

    Err(format!("Failed to get finalized status after {} attempts", MAX_RETRIES).into())
}

fn get_bundle_status(status_response: &serde_json::Value) -> Result<BundleStatus, Box<dyn std::error::Error>> {
    status_response
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(|value| value.as_array())
        .and_then(|statuses| statuses.get(0))
        .ok_or_else(|| format!("Failed to parse bundle status").into())
        .map(|bundle_status| BundleStatus {
            confirmation_status: bundle_status.get("confirmation_status").and_then(|s| s.as_str()).map(String::from),
            err: bundle_status.get("err").cloned(),
            transactions: bundle_status.get("transactions").and_then(|t| t.as_array()).map(|arr| {
                arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()
            }),
        })
}

fn check_transaction_error(bundle_status: &BundleStatus) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(err) = &bundle_status.err {
        if err["Ok"].is_null() {
            println!("Transaction executed without errors.");
            Ok(())
        } else {
            println!("Transaction encountered an error: {:?}", err);
            Err(format!("Transaction encountered an error").into())
        }
    } else {
        Ok(())
    }
}

fn print_transaction_url(bundle_status: &BundleStatus) {
    if let Some(transactions) = &bundle_status.transactions {
        if let Some(tx_id) = transactions.first() {
            println!("Transaction URL: https://solscan.io/tx/{}", tx_id);
        } else {
            println!("Unable to extract transaction ID.");
        }
    } else {
        println!("No transactions found in the bundle status.");
    }
}