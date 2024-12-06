use {
    keypair_utils::{get_or_create_keypair, get_or_create_keypair_elgamal, get_rpc_client, record_value}, solana_sdk::{
        signature::Keypair, signer::Signer, system_instruction::create_account, transaction::Transaction
    }, spl_token_2022::{extension::ExtensionType, instruction::initialize_mint, state::Mint}, spl_token_client::token::ExtensionInitializationParams, std::{error::Error, sync::Arc}
};

pub async fn create_mint(absolute_authority: &Keypair) -> Result<(), Box<dyn Error>> {
    let fee_payer_keypair = Arc::new(get_or_create_keypair("fee_payer_keypair")?);
    let client = get_rpc_client()?;
    let mint = get_or_create_keypair("mint")?;
    let mint_authority = absolute_authority;
    let freeze_authority = absolute_authority;
    let decimals = record_value("mint_decimals", 2)?;

    // Confidential Transfer Extension authority
    // Authority to modify the `ConfidentialTransferMint` configuration and to approve new accounts (if `auto_approve_new_accounts` is false?)
    let authority = absolute_authority;

    // Auditor ElGamal pubkey
    // Authority to decrypt any encrypted amounts for the mint
    let auditor_elgamal_keypair = get_or_create_keypair_elgamal("auditor_elgamal")?;

    // ConfidentialTransferMint extension parameters
    let confidential_transfer_mint_extension =
        ExtensionInitializationParams::ConfidentialTransferMint {
            authority: Some(authority.pubkey()),
            auto_approve_new_accounts: true, // If `true`, no approval is required and new accounts may be used immediately
            auditor_elgamal_pubkey: Some((*auditor_elgamal_keypair.pubkey()).into()),
        };

    // Calculate the space required for the mint account with the extension
    let space = ExtensionType::try_calculate_account_len::<Mint>(&[
        ExtensionType::ConfidentialTransferMint,
    ])?;

    // Calculate the lamports required for the mint account
    let rent = client.get_minimum_balance_for_rent_exemption(space)?;

    // Instructions to create the mint account
    let create_account_instruction = create_account(
        &fee_payer_keypair.pubkey(),
        &mint.pubkey(),
        rent,
        space as u64,
        &spl_token_2022::id(),
    );

    // ConfidentialTransferMint extension instruction
    let extension_instruction =
        confidential_transfer_mint_extension.instruction(&spl_token_2022::id(), &mint.pubkey())?;

    // Initialize the mint account
    //TODO: Use program-2022/src/extension/confidential_transfer/instruction/initialize_mint()
    let initialize_mint_instruction = initialize_mint(
        &spl_token_2022::id(),
        &mint.pubkey(),
        &mint_authority.pubkey(),
        Some(&freeze_authority.pubkey()),
        decimals,
    )?;

    let instructions = vec![
        create_account_instruction,
        extension_instruction,
        initialize_mint_instruction,
    ];

    let recent_blockhash = client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&fee_payer_keypair.pubkey()),
        &[&fee_payer_keypair, &mint as &dyn Signer],
        recent_blockhash,
    );
    let transaction_signature = client.send_and_confirm_transaction(&transaction)?;

    println!(
        "\nCreate Mint Account: https://explorer.solana.com/tx/{}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899",
        transaction_signature
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_mint() -> Result<(), Box<dyn Error>> {
        let absolute_authority = get_or_create_keypair("absolute_authority")?;
        assert!(create_mint(&absolute_authority).await.is_ok());
        Ok(())
    }
}