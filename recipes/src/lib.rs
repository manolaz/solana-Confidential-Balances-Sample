#[cfg(test)]
mod recipe {
    use std::error::Error;

    use keypair_utils::get_or_create_keypair;
    use setup_participants;
    use solana_sdk::signer::Signer;
    use transfer_public_mint;
    use setup_mint;
    use setup_token_account;
    use mint_tokens;
    use deposit_tokens;
    use apply_pending_balance;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn basic_transfer_recipe() -> Result<(), Box<dyn Error>> {
        let sender_keypair = get_or_create_keypair("sender_keypair")?;
        let recipient_keypair = get_or_create_keypair("recipient_keypair")?;
        let fee_payer_keypair = get_or_create_keypair("fee_payer_keypair")?;
        let absolute_mint_authority = get_or_create_keypair("absolute_mint_authority")?;
        

        // Step 1. Setup participants
        setup_participants::setup_basic_participant(&sender_keypair.pubkey()).await?;
        setup_participants::setup_basic_participant(&recipient_keypair.pubkey()).await?;
        setup_participants::setup_basic_participant(&fee_payer_keypair.pubkey()).await?;

        // Step 2. Create mint
        setup_mint::create_mint(&absolute_mint_authority).await?;
        
        // Step 3. Setup token account for sender
        setup_token_account::setup_token_account(&sender_keypair).await?;

        // Step 4. Mint tokens
        mint_tokens::mint_tokens(&absolute_mint_authority, &sender_keypair.pubkey(), 100_00).await?;
        
        // Step 5. Deposit tokens
        deposit_tokens::deposit_tokens(50_00, &sender_keypair).await?;

        // Step 6. Apply pending balance
        apply_pending_balance::apply_pending_balance(&sender_keypair).await?;
        
        // Step 7. Create recipient token account
        setup_token_account::setup_token_account(&recipient_keypair).await?;


        // Step X. Transfer tokens
        transfer_public_mint::main().await?;

        // signature = load_value("last_confidential_transfer_signature")
        // auditor_assert_transfer_amount(signature, assert_amount)
        
        Ok(())
    }

    // Add more recipes as needed, each with their own sequence of ingredients
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn another_recipe() -> Result<(), Box<dyn Error>> {
        // Different combination/order of ingredients
        //setup_participants::setup_basic_participant().await?;
        // ... other ingredients
        Ok(())
    }
}