use std::time::Duration;

use serde_json::json;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_zk_sdk::zk_elgamal_proof_program;
use spl_token_client::{client::{self, ProgramClient, SendTransaction, SimulateTransaction}, token::TokenError};

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
    let jito_sdk = JitoJsonRpcSDK::new("https://dallas.testnet.block-engine.jito.wtf/api/v1", None);
    let random_tip_account = jito_sdk.get_random_tip_account().await?;
    let jito_tip_account = Pubkey::from_str(&random_tip_account)?;
    let jito_tip_amount:u64 = get_max_jito_tip_amount().await?;
    println!("jito tip amount lamports: {}", jito_tip_amount);
    let jito_tip_ix = system_instruction::transfer(
        &sender_keypair.pubkey(),
        &jito_tip_account,
        jito_tip_amount,
    );

    

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
        &[range_create_ix, equality_create_ix, cv_create_ix, jito_tip_ix],
        Some(&sender_keypair.pubkey()),
        &[
            &sender_keypair, 
            &range_proof_context_state_account as &dyn Signer, 
            &equality_proof_context_state_account as &dyn Signer, 
            &ciphertext_validity_proof_context_state_account as &dyn Signer],
        client.get_latest_blockhash()?,
    );
    let serialized_tx1 = bs58::encode(bincode::serialize(&tx1)?).into_string();

    // Transaction 2: Encode Range Proof on its own (because it's the largest).
    let tx2 = Transaction::new_signed_with_payer(
        &[range_verify_ix],
        Some(&sender_keypair.pubkey()),
        &[&sender_keypair],
        client.get_latest_blockhash()?,
    );
    let serialized_tx2 = bs58::encode(bincode::serialize(&tx2)?).into_string();

    // Transaction 3: Encode all remaining proofs.
    let tx3 = Transaction::new_signed_with_payer(
        &[equality_verify_ix, cv_verify_ix],
        Some(&sender_keypair.pubkey()),
        &[&sender_keypair],
        client.get_latest_blockhash()?,
    );
    let serialized_tx3 = bs58::encode(bincode::serialize(&tx3)?).into_string();

    let tx_bundle = json!([serialized_tx1, serialized_tx2, serialized_tx3]);
    
    // UUID for the bundle
    let uuid = None;

    // Send bundle using Jito SDK
    println!("Sending bundle with X transactions...");
    let response = jito_sdk.send_bundle(Some(tx_bundle), uuid).await?;

    // Extract bundle UUID from response
    let bundle_uuid = response["result"]
        .as_str()
        .ok_or("Failed to get bundle UUID from response")?;
    println!("Bundle sent with UUID: {}", bundle_uuid);

    confirm_bundle_status(&jito_sdk, &bundle_uuid).await?;

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

    match token
        .confidential_transfer_transfer(
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
        )
        .await
    {
        Ok(RpcClientResponse::Signature(signature)) => {
            record_value("last_confidential_transfer_signature", signature.to_string())?;
            println!("\nTransfer with Split Proofs: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899", signature);
        }
        _ => {
            panic!("Unexpected result from transfer with split proofs");
        }
    }

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
    let transaction = Transaction::new_signed_with_payer(
        &[
            close_equality_proof_instruction,
            close_ciphertext_validity_proof_instruction,
            close_range_proof_instruction,
        ],
        Some(&sender_keypair.pubkey()),
        &[&sender_keypair], // Signers
        recent_blockhash,
    );

    let transaction_signature = client.send_and_confirm_transaction(&transaction)?;

    println!(
        "\nTransfer [Close Proof Accounts]: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899",
        transaction_signature
    );

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
    use solana_sdk::instruction::Instruction;

    let client = get_rpc_client()?;
    let space = size_of::<zk_elgamal_proof_program::state::ProofContextState<U>>();
    let rent = client.get_minimum_balance_for_rent_exemption(space)?;

    let context_state_info = ContextStateInfo {
        context_state_account: context_state_account_pubkey,
        context_state_authority: context_state_authority_pubkey,
    };

    let instruction_type = spl_token_confidential_transfer_proof_extraction::instruction::zk_proof_type_to_instruction(
        ZK::PROOF_TYPE,
    )?;

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

const max_retries: u32 = 30;
const retry_delay: Duration = Duration::from_secs(3);
async fn confirm_bundle_status(jito_sdk: &JitoJsonRpcSDK, bundle_uuid: &str) -> Result<(), Box<dyn std::error::Error>> {

    for attempt in 1..=max_retries {
        println!("Checking bundle status (attempt {}/{})", attempt, max_retries);

        let status_response = jito_sdk.get_in_flight_bundle_statuses(vec![bundle_uuid.to_string()]).await?;

        if let Some(result) = status_response.get("result") {
            if let Some(value) = result.get("value") {
                if let Some(statuses) = value.as_array() {
                    if let Some(bundle_status) = statuses.get(0) {
                        if let Some(status) = bundle_status.get("status") {
                            match status.as_str() {
                                Some("Landed") => {
                                    println!("Bundle landed on-chain. Checking final status...");
                                    return check_final_bundle_status(&jito_sdk, bundle_uuid).await;
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

        if attempt < max_retries {
            std::thread::sleep(retry_delay);
        }
    }

    Err(format!("Failed to confirm bundle status after {} attempts", max_retries).into())
}

async fn check_final_bundle_status(jito_sdk: &JitoJsonRpcSDK, bundle_uuid: &str) -> Result<(), Box<dyn std::error::Error>> {

    for attempt in 1..=max_retries {
        println!("Checking final bundle status (attempt {}/{})", attempt, max_retries);

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
                return Ok(());
            },
            Some(status) => {
                println!("Unexpected final bundle status: {}. Continuing to poll...", status);
            },
            None => {
                println!("Unable to parse final bundle status. Continuing to poll...");
            }
        }

        if attempt < max_retries {
            std::thread::sleep(retry_delay);
        }
    }

    Err(format!("Failed to get finalized status after {} attempts", max_retries).into())
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