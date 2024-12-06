use std::error::Error;

use keypair_utils::{get_or_create_keypair, get_rpc_client, load_value};
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token_2022::extension::confidential_transfer::instruction::deposit;

pub async fn deposit_tokens(deposit_amount: u64, depositor_keypair: &Keypair) -> Result<(), Box<dyn Error>> {
    let client = get_rpc_client()?;
    let mint = get_or_create_keypair("mint")?;
    let decimals = load_value("mint_decimals")?;

    // Confidential balance has separate "pending" and "available" balances
    // Must first deposit tokens from non-confidential balance to  "pending" confidential balance

    let depositor_token_account = get_associated_token_address_with_program_id(
        &depositor_keypair.pubkey(), // Token account owner
        &mint.pubkey(),        // Mint
        &spl_token_2022::id(),
    );

    // Instruction to deposit from non-confidential balance to "pending" balance
    let deposit_instruction = deposit(
        &spl_token_2022::id(),
        &depositor_token_account, // Token account
        &mint.pubkey(),                   // Mint
        deposit_amount,                   // Amount to deposit
        decimals,                         // Mint decimals
        &depositor_keypair.pubkey(),               // Token account owner
        &[&depositor_keypair.pubkey()],            // Signers
    )?;

    let recent_blockhash = client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[deposit_instruction],
        Some(&depositor_keypair.pubkey()),
        &[&depositor_keypair],
        recent_blockhash,
    );

    let transaction_signature = client.send_and_confirm_transaction(&transaction)?;

    println!(
        "\nDeposit Tokens: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899",
        transaction_signature
    );
    Ok(())
}