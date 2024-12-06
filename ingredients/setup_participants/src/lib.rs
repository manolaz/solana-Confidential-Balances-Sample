use {
    keypair_utils::{get_or_create_keypair, get_rpc_client},
    solana_sdk::{
        native_token::LAMPORTS_PER_SOL, signer::Signer,
    },
    std::{error::Error, sync::Arc},
    tokio,
};

pub async fn setup_basic_participants() -> Result<(), Box<dyn Error>> {


    // Step 1. Create sender and recipient wallet keypairs -----------------------------------
    let wallet_1 = Arc::new(get_or_create_keypair("wallet_1")?);
    let wallet_2 = Arc::new(get_or_create_keypair("wallet_2")?);

    let client = get_rpc_client()?;

    client.request_airdrop(&wallet_1.pubkey(), LAMPORTS_PER_SOL)?;
    client.request_airdrop(&wallet_2.pubkey(), LAMPORTS_PER_SOL)?;

    //Hack: To await airdrop settlement. Refactor to use async/await with appropriate commitment.
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_setup_basic_participants() {
        assert!(setup_basic_participants().await.is_ok());
    }
}
