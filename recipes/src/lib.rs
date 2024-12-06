#[cfg(test)]
mod recipe {
    use std::error::Error;

    use setup_participants;
    use transfer_public_mint;
    use setup_mint;
    use setup_token_account;
    use mint_tokens;
    use deposit_tokens;
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn basic_transfer_recipe() -> Result<(), Box<dyn Error>> {
        // Recipe: demonstrates a basic transfer flow
        setup_participants::setup_basic_participants().await?;
        setup_mint::create_mint().await?;
        setup_token_account::setup_token_account().await?;
        mint_tokens::mint_tokens().await?;
        deposit_tokens::deposit_tokens().await?;
        transfer_public_mint::main().await?;
        Ok(())
    }

    // Add more recipes as needed, each with their own sequence of ingredients
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn another_recipe() -> Result<(), Box<dyn Error>> {
        // Different combination/order of ingredients
        //setup_participants::setup_basic_participants().await?;
        // ... other ingredients
        Ok(())
    }
}