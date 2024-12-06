use std::{error::Error, sync::Arc};

use keypair_utils::{get_or_create_keypair, get_rpc_client, load_value, record_value};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signer::Signer, transaction::Transaction};
use spl_token_2022::instruction::mint_to;

pub async fn mint_tokens() -> Result<(), Box<dyn Error>> {
    let wallet_1 = Arc::new(get_or_create_keypair("wallet_1")?);
    let client = get_rpc_client()?;
    let mint = get_or_create_keypair("mint")?;
    let sender_associated_token_address: Pubkey = load_value("sender_associated_token_address")?;

    // Mint 100.00 tokens
    let amount = record_value("mint_amount", 100_00)?;

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
        client.get_latest_blockhash()?,
    );

    let transaction_signature = client.send_and_confirm_transaction(&transaction)?;

    println!(
        "\nMint Tokens: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899",
        transaction_signature
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mint_tokens() {
        mint_tokens().await.unwrap();
    }
}
