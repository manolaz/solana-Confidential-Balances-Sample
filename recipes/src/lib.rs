#[cfg(test)]
mod recipe {
    use std::error::Error;

    use apply_pending_balance;
    use deposit_tokens;
    use keypair_utils::{get_or_create_keypair, get_or_create_keypair_elgamal};
    use mint_tokens;
    use setup_mint;
    use setup_mint_confidential;
    use setup_participants;
    use setup_token_account;
    use solana_sdk::signer::Signer;
    use transfer;
    use withdraw_tokens;

    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn confidential_mintburn_transfer_recipe() -> Result<(), Box<dyn Error>> {
        let sender_keypair = get_or_create_keypair("sender_keypair")?;
        let recipient_keypair = get_or_create_keypair("recipient_keypair")?;
        let fee_payer_keypair = get_or_create_keypair("fee_payer_keypair")?;
        let auditor_elgamal_keypair = get_or_create_keypair_elgamal("auditor_elgamal")?;
        let absolute_mint_authority = get_or_create_keypair("absolute_mint_authority")?;

        // Step 1. Setup participants
        setup_participants::setup_basic_participant(&fee_payer_keypair.pubkey(), None).await?;
        setup_participants::setup_basic_participant(&sender_keypair.pubkey(), Some(&fee_payer_keypair)).await?;
        setup_participants::setup_basic_participant(&recipient_keypair.pubkey(), Some(&fee_payer_keypair)).await?;

        // Step 2. Create mint
        setup_mint_confidential::create_mint(&absolute_mint_authority, &auditor_elgamal_keypair).await?;

        // Step 3. Setup token account for sender
        setup_token_account::setup_token_account(&sender_keypair).await?;

        // Step 4. Confidentially mint tokens
        mint_tokens::go_with_confidential_mintburn(&absolute_mint_authority, &sender_keypair.pubkey(), 100_00, &auditor_elgamal_keypair).await?;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn basic_transfer_recipe() -> Result<(), Box<dyn Error>> {
        let sender_keypair = get_or_create_keypair("sender_keypair")?;
        let recipient_keypair = get_or_create_keypair("recipient_keypair")?;
        let fee_payer_keypair = get_or_create_keypair("fee_payer_keypair")?;
        let auditor_elgamal_keypair = get_or_create_keypair_elgamal("auditor_elgamal")?;
        let absolute_mint_authority = get_or_create_keypair("absolute_mint_authority")?;

        // Step 1. Setup participants
        setup_participants::setup_basic_participant(&fee_payer_keypair.pubkey(), None).await?;
        setup_participants::setup_basic_participant(&sender_keypair.pubkey(), Some(&fee_payer_keypair)).await?;
        setup_participants::setup_basic_participant(&recipient_keypair.pubkey(), Some(&fee_payer_keypair)).await?;

        // Step 2. Create mint
        setup_mint::create_mint(&absolute_mint_authority, &auditor_elgamal_keypair).await?;

        // Step 3. Setup token account for sender
        setup_token_account::setup_token_account(&sender_keypair).await?;

        // Step 4. Mint tokens
        mint_tokens::go(&absolute_mint_authority, &sender_keypair.pubkey(), 100_00).await?;

        // Step 5. Deposit tokens
        deposit_tokens::deposit_tokens(50_00, &sender_keypair).await?;

        // Step 6. Apply pending balance
        apply_pending_balance::apply_pending_balance(&sender_keypair).await?;

        // Step 7. Create recipient token account
        setup_token_account::setup_token_account(&recipient_keypair).await?;

        // Step 8. Transfer tokens with split proofs
        transfer::with_split_proofs(&sender_keypair, &recipient_keypair, 50_00).await?;

        // Step 9. Apply recipient's pending balance
        apply_pending_balance::apply_pending_balance(&recipient_keypair).await?;

        // Step 10. Withdraw tokens
        withdraw_tokens::withdraw_tokens(20_00, &recipient_keypair).await?;

        // Step 11. Auditor asserts last transfer amount
        global_auditor_assert::last_transfer_amount(50_00, &auditor_elgamal_keypair).await?;

        Ok(())
    }
}
