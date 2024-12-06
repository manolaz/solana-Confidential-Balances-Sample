use {
    keypair_utils::get_rpc_client,
    solana_sdk::{
        native_token::LAMPORTS_PER_SOL, pubkey::Pubkey
    },
    std::error::Error,
};

pub async fn setup_basic_participant(participant_pubkey: &Pubkey) -> Result<(), Box<dyn Error>> {

    let client = get_rpc_client()?;

    client.request_airdrop(&participant_pubkey, LAMPORTS_PER_SOL)?;

    //Hack: To await airdrop settlement. Refactor to use async/await with appropriate commitment.
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use keypair_utils::get_or_create_keypair;
    use solana_sdk::signer::Signer;

    use super::*;

    #[tokio::test]
    async fn test_setup_basic_participant() -> Result<(), Box<dyn Error>> {
        let participant_keypair = get_or_create_keypair("SOLO_TEST_participant_keypair")?;

        setup_basic_participant(&participant_keypair.pubkey()).await?;
        Ok(())
    }
}
