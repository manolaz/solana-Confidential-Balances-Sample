use std::error::Error;

use keypair_utils::{get_or_create_keypair, get_rpc_client};
use solana_sdk::{
    instruction::Instruction, pubkey::Pubkey, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token_2022::instruction::mint_to;

pub async fn mint_tokens(
    mint_authority: &Keypair,
    token_account_owner: &Pubkey,
    mint_amount: u64
) -> Result<(), Box<dyn Error>> {
    let client = get_rpc_client()?;
    let mint = get_or_create_keypair("mint")?;
    let fee_payer_keypair = get_or_create_keypair("fee_payer_keypair")?;

    let receiving_token_account = get_associated_token_address_with_program_id(
        &token_account_owner, // Token account owner
        &mint.pubkey(),        // Mint
        &spl_token_2022::id(),
    );

    // Instruction to mint tokens
    let mint_to_instruction: Instruction = mint_to(
        &spl_token_2022::id(),
        &mint.pubkey(),              // Mint
        &receiving_token_account,     // Token account to mint to
        &mint_authority.pubkey(),         // Token account owner
        &[&mint_authority.pubkey()], // Additional signers (mint authority)
        mint_amount,                      // Amount to mint
    )?;

    let transaction = Transaction::new_signed_with_payer(
        &[mint_to_instruction],
        Some(&fee_payer_keypair.pubkey()),
        &[&fee_payer_keypair, &mint_authority],
        client.get_latest_blockhash()?,
    );

    let transaction_signature = client.send_and_confirm_transaction(&transaction)?;

    println!(
        "\nMint Tokens: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899",
        transaction_signature
    );
    Ok(())
}