// cargo run --bin main
use {
    keypair_utils::{get_or_create_keypair, get_rpc_client, load_value},
    simple_logger::SimpleLogger,
    solana_client::nonblocking::rpc_client::RpcClient as NonBlockingRpcClient,
    solana_sdk::{
        commitment_config::CommitmentConfig,
        signature::{Keypair, Signer},
        transaction::Transaction,
    },
    spl_associated_token_account::
        get_associated_token_address_with_program_id
    ,
    spl_token_2022::{
        error::TokenError,
        extension::{
            confidential_transfer::{
                account_info::{
                    ApplyPendingBalanceAccountInfo, WithdrawAccountInfo,
                },
                instruction::
                    apply_pending_balance
                ,
                ConfidentialTransferAccount,
            },
            BaseStateWithExtensions,
        },
        solana_zk_sdk::
            encryption::{
                auth_encryption::AeKey,
                elgamal::ElGamalKeypair,
            }
        ,
    },
    spl_token_client::{
        client::{ProgramRpcClient, ProgramRpcClientSendTransaction, RpcClientResponse},
        token::{ProofAccount, Token},
    },
    spl_token_confidential_transfer_proof_generation::
        withdraw::WithdrawProofData
    ,
    std::{error::Error, sync::Arc},
};

pub async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize the logger with the trace level
    SimpleLogger::new()
        .with_level(log::LevelFilter::Error)
        .init()
        .unwrap();

    // Step 1. Setup Participants -------------------------------------------------
    // Step 2. Setup Mint -------------------------------------------------
    // Step 3. Setup Token Account -------------------------------------------------
    // Step 4. Mint Tokens ----------------------------------------------------------
    // Step 5. Deposit Tokens -------------------------------------------------------
    // Step 6. Apply Sender's Pending Balance -------------------------------------------------
    // Step 7. Create Recipient Token Account -----------------------------------------
    // Step 8. Transfer with ZK Proofs ---------------------------------------------------
    // Step 9. Apply Pending Balance -------------------------------------------------
    let client = get_rpc_client()?;
    let mint = get_or_create_keypair("mint")?;
    let decimals = load_value("decimals")?;
    let recipient_keypair = Arc::new(get_or_create_keypair("recipient_keypair")?);
    let recipient_associated_token_address = get_associated_token_address_with_program_id(
        &recipient_keypair.pubkey(),
        &mint.pubkey(),
        &spl_token_2022::id(),
    );

    // The "pending" balance must be applied to "available" balance before it can be withdrawn

    // A "non-blocking" RPC client (for async calls)
    let rpc_client = NonBlockingRpcClient::new_with_commitment(
        String::from("http://127.0.0.1:8899"),
        CommitmentConfig::confirmed(),
    );

    let program_client =
        ProgramRpcClient::new(Arc::new(rpc_client), ProgramRpcClientSendTransaction);

    // Create a "token" client, to use various helper functions for Token Extensions
    let token = Token::new(
        Arc::new(program_client),
        &spl_token_2022::id(),
        &mint.pubkey(),
        Some(decimals),
        recipient_keypair.clone(),
    );

    // Get receiver token account data
    let token_account_info = token
        .get_account_info(&recipient_associated_token_address)
        .await?;

    // Unpack the ConfidentialTransferAccount extension portion of the token account data
    let confidential_transfer_account =
        token_account_info.get_extension::<ConfidentialTransferAccount>()?;

    // ConfidentialTransferAccount extension information needed to construct an `ApplyPendingBalance` instruction.
    let apply_pending_balance_account_info =
        ApplyPendingBalanceAccountInfo::new(confidential_transfer_account);

    // Return the number of times the pending balance has been credited
    let expected_pending_balance_credit_counter =
        apply_pending_balance_account_info.pending_balance_credit_counter();

    // Create the ElGamal keypair and AES key for the recipient token account
    let receiver_elgamal_keypair =
        ElGamalKeypair::new_from_signer(&recipient_keypair, &recipient_associated_token_address.to_bytes())
            .unwrap();
    let receiver_aes_key =
        AeKey::new_from_signer(&recipient_keypair, &recipient_associated_token_address.to_bytes()).unwrap();

    // Update the decryptable available balance (add pending balance to available balance)
    let new_decryptable_available_balance = apply_pending_balance_account_info
        .new_decryptable_available_balance(&receiver_elgamal_keypair.secret(), &receiver_aes_key)
        .map_err(|_| TokenError::AccountDecryption)?;

    // Create a `ApplyPendingBalance` instruction
    let apply_pending_balance_instruction = apply_pending_balance(
        &spl_token_2022::id(),
        &recipient_associated_token_address,      // Token account
        expected_pending_balance_credit_counter, // Expected number of times the pending balance has been credited
        new_decryptable_available_balance.into(), // Cipher text of the new decryptable available balance
        &recipient_keypair.pubkey(),                       // Token account owner
        &[&recipient_keypair.pubkey()],                    // Additional signers
    )?;

    let recent_blockhash = client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[apply_pending_balance_instruction],
        Some(&recipient_keypair.pubkey()),
        &[&recipient_keypair],
        recent_blockhash,
    );

    let transaction_signature = client.send_and_confirm_transaction(&transaction)?;

    println!(
        "\nApply Pending Balance (to recipient): https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899",
        transaction_signature
    );

    // Step 10. Withdraw Tokens ------------------------------------------------------

    let withdraw_amount = 20_00;

    // Get recipient token account data
    let token_account = token
        .get_account_info(&recipient_associated_token_address)
        .await?;

    // Unpack the ConfidentialTransferAccount extension portion of the token account data
    let extension_data = token_account.get_extension::<ConfidentialTransferAccount>()?;

    // Confidential Transfer extension information needed to construct a `Withdraw` instruction.
    let withdraw_account_info = WithdrawAccountInfo::new(extension_data);

    // Authority for the withdraw proof account (to close the account)
    let context_state_authority = &recipient_keypair;

    let equality_proof_context_state_keypair = Keypair::new();
    let equality_proof_context_state_pubkey = equality_proof_context_state_keypair.pubkey();
    let range_proof_context_state_keypair = Keypair::new();
    let range_proof_context_state_pubkey = range_proof_context_state_keypair.pubkey();

    // Create a withdraw proof data
    let WithdrawProofData {
        equality_proof_data,
        range_proof_data,
    } = withdraw_account_info.generate_proof_data(
        withdraw_amount,
        &receiver_elgamal_keypair,
        &receiver_aes_key,
    )?;

    // Generate withdrawal proof accounts
    let context_state_authority_pubkey = context_state_authority.pubkey();
    let create_equality_proof_signer = &[&equality_proof_context_state_keypair];
    let create_range_proof_signer = &[&range_proof_context_state_keypair];

    match token
        .confidential_transfer_create_context_state_account(
            &equality_proof_context_state_pubkey,
            &context_state_authority_pubkey,
            &equality_proof_data,
            false,
            create_equality_proof_signer,
        )
        .await
    {
        Ok(RpcClientResponse::Signature(signature)) => {
            println!("Equality Proof Context State Account: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899", signature);
        }
        _ => {
            panic!("Unexpected result from create equality proof context state account");
        }
    }
    match token
        .confidential_transfer_create_context_state_account(
            &range_proof_context_state_pubkey,
            &context_state_authority_pubkey,
            &range_proof_data,
            true,
            create_range_proof_signer,
        )
        .await
    {
        Ok(RpcClientResponse::Signature(signature)) => {
            println!("Range Proof Context State Account: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899", signature);
        }
        _ => {
            panic!("Unexpected result from create range proof context state account");
        }
    }

    // do the withdrawal
    match token
        .confidential_transfer_withdraw(
            &recipient_associated_token_address,
            &recipient_keypair.pubkey(),
            Some(&ProofAccount::ContextAccount(
                equality_proof_context_state_pubkey,
            )),
            Some(&ProofAccount::ContextAccount(
                range_proof_context_state_pubkey,
            )),
            withdraw_amount,
            decimals,
            Some(withdraw_account_info),
            &receiver_elgamal_keypair,
            &receiver_aes_key,
            &[&recipient_keypair],
        )
        .await
    {
        Ok(RpcClientResponse::Signature(signature)) => {
            println!("Withdraw Transaction: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899", signature);
        }
        Ok(RpcClientResponse::Transaction(_)) => {
            panic!("Unexpected result from withdraw: transaction");
        }
        Ok(RpcClientResponse::Simulation(_)) => {
            panic!("Unexpected result from withdraw: simulation");
        }
        Err(e) => {
            panic!("Unexpected result from withdraw: {:?}", e);
        }
    }

    // close context state account
    let close_context_state_signer = &[&context_state_authority];

    match token
        .confidential_transfer_close_context_state_account(
            &equality_proof_context_state_pubkey,
            &recipient_associated_token_address,
            &context_state_authority_pubkey,
            close_context_state_signer,
        )
        .await
    {
        Ok(RpcClientResponse::Signature(signature)) => {
            println!("Close Equality Proof Context State Account: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899", signature);
        }
        _ => {
            panic!("Unexpected result from close equality proof context state account");
        }
    }
    match token
        .confidential_transfer_close_context_state_account(
            &range_proof_context_state_pubkey,
            &recipient_associated_token_address,
            &context_state_authority_pubkey,
            close_context_state_signer,
        )
        .await
    {
        Ok(RpcClientResponse::Signature(signature)) => {
            println!("Close Range Proof Context State Account: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899", signature);
        }
        _ => {
            panic!("Unexpected result from close range proof context state account");
        }
    }

    Ok(())
}

#[cfg(test)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_main() -> Result<(), Box<dyn Error>> {
    main().await
}
