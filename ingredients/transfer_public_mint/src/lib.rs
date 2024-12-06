// cargo run --bin main
use {
    keypair_utils::{get_or_create_keypair, get_rpc_client, load_value}, simple_logger::SimpleLogger, solana_client::nonblocking::rpc_client::RpcClient as NonBlockingRpcClient, solana_sdk::{
        commitment_config::CommitmentConfig,
        instruction::Instruction,
        signature::{Keypair, Signer},
        transaction::Transaction,
    }, spl_associated_token_account::{
        get_associated_token_address_with_program_id, 
        instruction::create_associated_token_account,
    }, spl_token_2022::{
        error::TokenError,
        extension::{
            confidential_transfer::{
                account_info::{
                    ApplyPendingBalanceAccountInfo, 
                    TransferAccountInfo, 
                    WithdrawAccountInfo,
                },
                instruction::{
                    apply_pending_balance, 
                    configure_account, 
                    deposit, 
                    PubkeyValidityProofData,
                },
                ConfidentialTransferAccount, 
                ConfidentialTransferMint,
            },
            BaseStateWithExtensions, 
            ExtensionType, 
            StateWithExtensionsOwned,
        },
        instruction::{mint_to, reallocate},
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
    }, spl_token_confidential_transfer_proof_extraction::instruction::{ProofData, ProofLocation}, spl_token_confidential_transfer_proof_generation::{
        transfer::TransferProofData, 
        withdraw::WithdrawProofData,
    }, std::{error::Error, sync::Arc}
};

pub async fn main() -> Result<(), Box<dyn Error>> {

    // Initialize the logger with the trace level
    SimpleLogger::new().with_level(log::LevelFilter::Error).init().unwrap();

    // Step 3. Create Sender Token Account -------------------------------------------
    let wallet_1 = Arc::new(get_or_create_keypair("wallet_1")?);
    let wallet_2 = Arc::new(get_or_create_keypair("wallet_2")?);
    let client = get_rpc_client()?;
    let mint = get_or_create_keypair("mint")?;
    let decimals = load_value("decimals")?;
    
    // Associated token address of the sender
    let sender_associated_token_address = get_associated_token_address_with_program_id(
        &wallet_1.pubkey(), // Token account owner
        &mint.pubkey(),     // Mint
        &spl_token_2022::id(),
    );

    // Instruction to create associated token account
    let create_associated_token_account_instruction = create_associated_token_account(
        &wallet_1.pubkey(), // Funding account
        &wallet_1.pubkey(), // Token account owner
        &mint.pubkey(),     // Mint
        &spl_token_2022::id(),
    );

    // Instruction to reallocate the token account to include the `ConfidentialTransferAccount` extension
    let reallocate_instruction = reallocate(
        &spl_token_2022::id(),
        &sender_associated_token_address, // Token account
        &wallet_1.pubkey(),               // Payer
        &wallet_1.pubkey(),               // Token account owner
        &[&wallet_1.pubkey()],            // Signers
        &[ExtensionType::ConfidentialTransferAccount], // Extension to reallocate space for
    )?;

    // Create the ElGamal keypair and AES key for the sender token account
    let elgamal_keypair =
        ElGamalKeypair::new_from_signer(&wallet_1, &sender_associated_token_address.to_bytes())
            .unwrap();
    let aes_key =
        AeKey::new_from_signer(&wallet_1, &sender_associated_token_address.to_bytes()).unwrap();

    // The maximum number of `Deposit` and `Transfer` instructions that can
    // credit `pending_balance` before the `ApplyPendingBalance` instruction is executed
    let maximum_pending_balance_credit_counter = 65536;

    // Initial token balance is 0
    let decryptable_balance = aes_key.encrypt(0);

    // The instruction data that is needed for the `ProofInstruction::VerifyPubkeyValidity` instruction.
    // It includes the cryptographic proof as well as the context data information needed to verify the proof.
    // Generating the proof data client-side (instead of using a separate proof account)
    let proof_data =
        PubkeyValidityProofData::new(&elgamal_keypair).map_err(|_| TokenError::ProofGeneration)?;

    // `InstructionOffset` indicates that proof is included in the same transaction
    // This means that the proof instruction offset must be always be 1.
    let proof_location = ProofLocation::InstructionOffset(
        1.try_into().unwrap(), 
        ProofData::InstructionData(&proof_data)
    );

    // Instructions to configure the token account, including the proof instruction
    // Appends the `VerifyPubkeyValidityProof` instruction right after the `ConfigureAccount` instruction.
    let configure_account_instruction = configure_account(
        &spl_token_2022::id(),                  // Program ID
        &&sender_associated_token_address,      // Token account
        &mint.pubkey(),                         // Mint
        decryptable_balance.into(),                    // Initial balance
        maximum_pending_balance_credit_counter, // Maximum pending balance credit counter
        &wallet_1.pubkey(),                     // Token Account Owner
        &[],                                    // Additional signers
        proof_location,                         // Proof location
    )
    .unwrap();

    // Instructions to configure account must come after `initialize_account` instruction
    let mut instructions = vec![
        create_associated_token_account_instruction,
        reallocate_instruction,
    ];
    instructions.extend(configure_account_instruction);

    let recent_blockhash = client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&wallet_1.pubkey()),
        &[&wallet_1],
        recent_blockhash,
    );

    let transaction_signature = client.send_and_confirm_transaction(&transaction)?;

    println!(
        "\nCreate Sender Token Account: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899",
        transaction_signature
    );

    // Step 4. Mint Tokens ----------------------------------------------------------

    // Mint 100.00 tokens
    let amount = 100_00;

    // Instruction to mint tokens
    let mint_to_instruction: Instruction = mint_to(
        &spl_token_2022::id(),
        &mint.pubkey(),                   // Mint
        &sender_associated_token_address, // Token account to mint to
        &wallet_1.pubkey(),               // Token account owner
        &[&wallet_1.pubkey()],            // Additional signers (mint authority)
        amount,                           // Amount to mint
    )?;

    let transaction = Transaction::new_signed_with_payer(
        &[mint_to_instruction],
        Some(&wallet_1.pubkey()),
        &[&wallet_1],
        recent_blockhash,
    );

    let transaction_signature = client.send_and_confirm_transaction(&transaction)?;

    println!(
        "\nMint Tokens: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899",
        transaction_signature
    );

    // Step 5. Deposit Tokens -------------------------------------------------------

    // Confidential balance has separate "pending" and "available" balances
    // Must first deposit tokens from non-confidential balance to  "pending" confidential balance

    // Amount to deposit, 50.00 tokens
    let deposit_amount = 50_00;

    // Instruction to deposit from non-confidential balance to "pending" balance
    let deposit_instruction = deposit(
        &spl_token_2022::id(),
        &sender_associated_token_address, // Token account
        &mint.pubkey(),                   // Mint
        deposit_amount,                   // Amount to deposit
        decimals,                         // Mint decimals
        &wallet_1.pubkey(),               // Token account owner
        &[&wallet_1.pubkey()],            // Signers
    )?;

    let recent_blockhash = client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[deposit_instruction],
        Some(&wallet_1.pubkey()),
        &[&wallet_1],
        recent_blockhash,
    );

    let transaction_signature = client.send_and_confirm_transaction(&transaction)?;

    println!(
        "\nDeposit Tokens: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899",
        transaction_signature
    );

    // Step 6. Apply Sender's Pending Balance -------------------------------------------------

    // The "pending" balance must be applied to "available" balance before it can be transferred

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
        wallet_1.clone(),
    );

    // Get sender token account data
    let token_account_info = token
        .get_account_info(&sender_associated_token_address)
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

    // Update the decryptable available balance (add pending balance to available balance)
    let new_decryptable_available_balance = apply_pending_balance_account_info
        .new_decryptable_available_balance(&elgamal_keypair.secret(), &aes_key)
        .map_err(|_| TokenError::AccountDecryption)?;

    // Create a `ApplyPendingBalance` instruction
    let apply_pending_balance_instruction = apply_pending_balance(
        &spl_token_2022::id(),
        &sender_associated_token_address,        // Token account
        expected_pending_balance_credit_counter, // Expected number of times the pending balance has been credited
        new_decryptable_available_balance.into(), // Cipher text of the new decryptable available balance
        &wallet_1.pubkey(),                // Token account owner
        &[&wallet_1.pubkey()],             // Additional signers
    )?;

    let recent_blockhash = client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[apply_pending_balance_instruction],
        Some(&wallet_1.pubkey()),
        &[&wallet_1],
        recent_blockhash,
    );

    let transaction_signature = client.send_and_confirm_transaction(&transaction)?;

    println!(
        "\nApply Pending Balance: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899",
        transaction_signature
    );

    // Step 7. Create Recipient Token Account -----------------------------------------

    // Associated token address of the recipient
    let recipient_associated_token_address = get_associated_token_address_with_program_id(
        &wallet_2.pubkey(), // Token account owner
        &mint.pubkey(),     // Mint
        &spl_token_2022::id(),
    );

    // Instruction to create associated token account
    let create_associated_token_account_instruction = create_associated_token_account(
        &wallet_2.pubkey(), // Funding account
        &wallet_2.pubkey(), // Token account owner
        &mint.pubkey(),     // Mint
        &spl_token_2022::id(),
    );

    // Instruction to reallocate the token account to include the `ConfidentialTransferAccount` extension
    let reallocate_instruction = reallocate(
        &spl_token_2022::id(),
        &recipient_associated_token_address,
        &wallet_2.pubkey(),    // payer
        &wallet_2.pubkey(),    // owner
        &[&wallet_2.pubkey()], // signers
        &[ExtensionType::ConfidentialTransferAccount],
    )?;

    // Create the ElGamal keypair and AES key for the recipient token account
    let elgamal_keypair =
        ElGamalKeypair::new_from_signer(&wallet_2, &recipient_associated_token_address.to_bytes())
            .unwrap();
    let aes_key =
        AeKey::new_from_signer(&wallet_2, &recipient_associated_token_address.to_bytes()).unwrap();

    let maximum_pending_balance_credit_counter = 65536; // Default value or custom
    let decryptable_balance = aes_key.encrypt(0);

    // Create proof data for Pubkey Validity
    let proof_data =
        PubkeyValidityProofData::new(&elgamal_keypair).map_err(|_| TokenError::ProofGeneration)?;

    // The proof is included in the same transaction of a corresponding token-2022 instruction
    // Appends the proof instruction right after the `ConfigureAccount` instruction.
    // This means that the proof instruction offset must be always be 1.
    let proof_location = ProofLocation::InstructionOffset(1.try_into().unwrap(), ProofData::InstructionData(&proof_data));

    // Configure account with the proof
    let configure_account_instruction = configure_account(
        &spl_token_2022::id(),
        &recipient_associated_token_address,
        &mint.pubkey(),
        decryptable_balance.into(),
        maximum_pending_balance_credit_counter,
        &wallet_2.pubkey(),
        &[],
        proof_location,
    )
    .unwrap();

    let mut instructions = vec![
        create_associated_token_account_instruction,
        reallocate_instruction,
    ];
    instructions.extend(configure_account_instruction);

    let recent_blockhash = client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&wallet_2.pubkey()),
        &[&wallet_2],
        recent_blockhash,
    );

    let transaction_signature = client.send_and_confirm_transaction(&transaction)?;

    println!(
        "\nCreate Recipient Token Account: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899",
        transaction_signature
    );

    // Step 8. Transfer with ZK Proofs ---------------------------------------------------

    // Must first create 3 accounts to store proofs before transferring tokens
    // This must be done in a separate transactions because the proofs are too large for single transaction

    // Equality Proof - prove that two ciphertexts encrypt the same value
    // Ciphertext Validity Proof - prove that ciphertexts are properly generated
    // Range Proof - prove that ciphertexts encrypt a value in a specified range (0, u64::MAX)

    // "Authority" for the proof accounts (to close the accounts after the transfer)
    let context_state_authority = &wallet_1;

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

    let sender_account_extension_data = sender_token_account_info.get_extension::<ConfidentialTransferAccount>()?;

    // ConfidentialTransferAccount extension information needed to create proof data
    let sender_transfer_account_info = TransferAccountInfo::new(sender_account_extension_data);

    let transfer_amount = 50_00;

    let sender_elgamal_keypair =
        ElGamalKeypair::new_from_signer(&wallet_1, &sender_associated_token_address.to_bytes())?;
    let sender_aes_key =
        AeKey::new_from_signer(&wallet_1, &sender_associated_token_address.to_bytes())?;

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
        transfer_amount,
        &sender_elgamal_keypair,
        &sender_aes_key,
        &recipient_elgamal_pubkey,
        Some(&auditor_elgamal_pubkey)
    )?;

    // Create 3 proofs ------------------------------------------------------

    // Range Proof ------------------------------------------------------------------------------

    //TODO: splitting proofs into separate txns means we don't get the signature of the txn that creates the proof accounts
    // It checks if failed, but we don't know which txn failed.
    match token
        .confidential_transfer_create_context_state_account(
            &range_proof_context_state_account.pubkey(),
            &context_state_authority.pubkey(),
            &range_proof_data,
            true, // Sent as separate transactions because proof instruction is too large.
            &[&range_proof_context_state_account],
        )
        .await {
            Ok(RpcClientResponse::Signature(signature)) => {
                println!("\nCreate Range Proof Context State: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899", signature);
            }
            Ok(RpcClientResponse::Transaction(_)) => {
                panic!("Unexpected result from create range proof context state account.");
            }
            Ok(RpcClientResponse::Simulation(_)) => {
                panic!("Unexpected result from create range proof context state account.");
            }
            Err(e) => {
                panic!("Unexpected result from create range proof context state account: {:?}", e);
            }
        }

    // Equality Proof ---------------------------------------------------------------------------

    match token.confidential_transfer_create_context_state_account(
        &equality_proof_pubkey,
        &context_state_authority.pubkey(),
        &equality_proof_data,
        false,
        &[&equality_proof_context_state_account],
    ).await {
        Ok(RpcClientResponse::Signature(signature)) => {
            println!("\nCreate Equality Proof Context State: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899", signature);
        }
        _ => {
            panic!("Unexpected result from create equality proof context state account");
        }
    }

    // Ciphertext Validity Proof ----------------------------------------------------------------

    match token.confidential_transfer_create_context_state_account(
        &ciphertext_validity_proof_pubkey,
        &context_state_authority.pubkey(),
        &ciphertext_validity_proof_data_with_ciphertext.proof_data,
        false,
        &[&ciphertext_validity_proof_context_state_account],
    ).await {
        Ok(RpcClientResponse::Signature(signature)) => {
            println!("\nCreate Ciphertext Validity Proof Context State: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899", signature);
        }
        _ => {
            panic!("Unexpected result from create ciphertext validity proof context state account");
        }
    }

    // Transfer with Split Proofs -------------------------------------------

    let equality_proof_context_proof_account =
        ProofAccount::ContextAccount(equality_proof_pubkey);
    let ciphertext_validity_proof_context_proof_account =
        ProofAccount::ContextAccount(ciphertext_validity_proof_pubkey);
    let range_proof_context_proof_account =
        ProofAccount::ContextAccount(range_proof_pubkey);

    let ciphertext_validity_proof_account_with_ciphertext = ProofAccountWithCiphertext {
        proof_account: ciphertext_validity_proof_context_proof_account,
        ciphertext_lo: ciphertext_validity_proof_data_with_ciphertext.ciphertext_lo,
        ciphertext_hi: ciphertext_validity_proof_data_with_ciphertext.ciphertext_hi,
    };

    match token
    .confidential_transfer_transfer(
        &sender_associated_token_address,
        &recipient_associated_token_address,
        &wallet_1.pubkey(),
        Some(&equality_proof_context_proof_account),
        Some(&ciphertext_validity_proof_account_with_ciphertext),
        Some(&range_proof_context_proof_account),
        transfer_amount,
        Some(sender_transfer_account_info),
        &sender_elgamal_keypair,
        &sender_aes_key,
        &recipient_elgamal_pubkey,
        Some(&auditor_elgamal_pubkey),
        &[&wallet_1],
    )
    .await {
        Ok(RpcClientResponse::Signature(signature)) => {
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
    let destination_account = &wallet_1.pubkey();

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
        Some(&wallet_1.pubkey()),
        &[&wallet_1], // Signers
        recent_blockhash,
    );

    let transaction_signature = client.send_and_confirm_transaction(&transaction)?;

    println!(
        "\nClose Proof Accounts: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899",
        transaction_signature
    );

    // Step 9. Apply Pending Balance -------------------------------------------------

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
        wallet_2.clone(),
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
        ElGamalKeypair::new_from_signer(&wallet_2, &recipient_associated_token_address.to_bytes())
            .unwrap();
    let receiver_aes_key =
        AeKey::new_from_signer(&wallet_2, &recipient_associated_token_address.to_bytes())
            .unwrap();

    // Update the decryptable available balance (add pending balance to available balance)
    let new_decryptable_available_balance = apply_pending_balance_account_info
        .new_decryptable_available_balance(&receiver_elgamal_keypair.secret(), &receiver_aes_key)
        .map_err(|_| TokenError::AccountDecryption)?;

    // Create a `ApplyPendingBalance` instruction
    let apply_pending_balance_instruction = apply_pending_balance(
        &spl_token_2022::id(),
        &recipient_associated_token_address,        // Token account
        expected_pending_balance_credit_counter, // Expected number of times the pending balance has been credited
        new_decryptable_available_balance.into(), // Cipher text of the new decryptable available balance
        &wallet_2.pubkey(),                // Token account owner
        &[&wallet_2.pubkey()],             // Additional signers
    )?;

    let recent_blockhash = client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[apply_pending_balance_instruction],
        Some(&wallet_2.pubkey()),
        &[&wallet_2],
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
    let context_state_authority = &wallet_2;

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

    match token.confidential_transfer_create_context_state_account(
        &equality_proof_context_state_pubkey,
        &context_state_authority_pubkey,
        &equality_proof_data,
        false,
        create_equality_proof_signer
    ).await {
        Ok(RpcClientResponse::Signature(signature)) => {
            println!("Equality Proof Context State Account: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899", signature);
        }
        _ => {
            panic!("Unexpected result from create equality proof context state account");
        }
    }
    match token.confidential_transfer_create_context_state_account(
        &range_proof_context_state_pubkey,
        &context_state_authority_pubkey,
        &range_proof_data,
        true,
        create_range_proof_signer,
    ).await {
        Ok(RpcClientResponse::Signature(signature)) => {
            println!("Range Proof Context State Account: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899", signature);
        }
        _ => {
            panic!("Unexpected result from create range proof context state account");
        }
    }

    // do the withdrawal
    match token.confidential_transfer_withdraw(
        &recipient_associated_token_address,
        &wallet_2.pubkey(),
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
        &[&wallet_2],
    ).await {
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

    match token.confidential_transfer_close_context_state_account(
        &equality_proof_context_state_pubkey,
        &recipient_associated_token_address,
        &context_state_authority_pubkey,
        close_context_state_signer
    ).await {
        Ok(RpcClientResponse::Signature(signature)) => {
            println!("Close Equality Proof Context State Account: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899", signature);
        }
        _ => {
            panic!("Unexpected result from close equality proof context state account");
        }
    }
    match token.confidential_transfer_close_context_state_account(
        &range_proof_context_state_pubkey,
        &recipient_associated_token_address,
        &context_state_authority_pubkey,
        close_context_state_signer
    ).await {
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