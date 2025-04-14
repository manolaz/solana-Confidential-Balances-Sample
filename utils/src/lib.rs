use gcp::GcpSigner;
use solana_client::nonblocking::rpc_client::RpcClient as NonBlockingRpcClient;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::signer::Signer;
use solana_zk_sdk::encryption::auth_encryption::AeKey;
use solana_zk_sdk::encryption::elgamal::{ElGamalKeypair, ElGamalSecretKey};
use std::env;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use dotenvy;

pub mod gcp;
pub mod jito;

pub const ENV_FILE_PATH: &str = "../.env";
pub const RUNTIME_ENV_FILE_PATH: &str = "../runtime_output.env";

// Get or create a keypair from an .env file
pub fn get_or_create_keypair(variable_name: &str) -> Result<Keypair, Box<dyn Error>> {
    // First check runtime_output.env if it exists
    if std::path::Path::new(RUNTIME_ENV_FILE_PATH).exists() {
        dotenvy::from_filename_override(RUNTIME_ENV_FILE_PATH).ok();
        if let Ok(secret_key_string) = env::var(variable_name) {
            // Try to parse from runtime_output.env
            let decoded_secret_key: Vec<u8> = serde_json::from_str(&secret_key_string)?;
            return Ok(Keypair::from_bytes(&decoded_secret_key)?);
        }
    }
    
    // Then check original .env
    dotenvy::from_filename_override(ENV_FILE_PATH).ok();

    match env::var(variable_name) {
        Ok(secret_key_string) => {
            // Parse from .env
            let decoded_secret_key: Vec<u8> = serde_json::from_str(&secret_key_string)?;
            Ok(Keypair::from_bytes(&decoded_secret_key)?)
        }
        Err(_) => {
            // Create a new keypair if the environment variable is not found in either file
            let keypair = Keypair::new();

            // Convert secret key to Vec<u8> and then to JSON, append to runtime_output.env file
            let secret_key_bytes = Vec::from(keypair.to_bytes());
            let json_secret_key = serde_json::to_string(&secret_key_bytes)?;

            // Create runtime_output.env if it doesn't exist
            if !std::path::Path::new(RUNTIME_ENV_FILE_PATH).exists() {
                std::fs::File::create(RUNTIME_ENV_FILE_PATH)?;
            }

            // Open runtime_output.env file, create it if it does not exist
            let mut file = OpenOptions::new().append(true).create(true).open(RUNTIME_ENV_FILE_PATH)?;

            writeln!(file, "{}={}", variable_name, json_secret_key)?;

            Ok(keypair)
        }
    }
}
pub fn get_turnkey_signer(private_key_id_env: &str, public_key_env: &str) -> Result<Box<dyn Signer + Send>, Box<dyn Error>> {
    let signer = tk_rs::TurnkeySigner::new(
        dotenvy::var("TURNKEY_API_PUBLIC_KEY").unwrap(),
        dotenvy::var("TURNKEY_API_PRIVATE_KEY").unwrap(),
        dotenvy::var("TURNKEY_ORGANIZATION_ID").unwrap(),
        dotenvy::var(private_key_id_env).unwrap(),
        dotenvy::var(public_key_env).unwrap(),
    )?;
    Ok(Box::new(signer))
}


pub fn get_or_create_keypair_elgamal(variable_name: &str) -> Result<ElGamalKeypair, Box<dyn Error>> {
    // First check runtime_output.env if it exists
    if std::path::Path::new(RUNTIME_ENV_FILE_PATH).exists() {
        dotenvy::from_filename_override(RUNTIME_ENV_FILE_PATH).ok();
        if let Ok(secret_key_string) = env::var(variable_name) {
            // Try to parse from runtime_output.env
            let decoded_secret_key: Vec<u8> = serde_json::from_str(&secret_key_string)?;
            return Ok(ElGamalKeypair::new(ElGamalSecretKey::from_seed(&decoded_secret_key)?));
        }
    }
    
    // Then check original .env
    dotenvy::from_filename_override(ENV_FILE_PATH).ok();

    match env::var(variable_name) {
        Ok(secret_key_string) => {
            let decoded_secret_key: Vec<u8> = serde_json::from_str(&secret_key_string)?;
            Ok(ElGamalKeypair::new(ElGamalSecretKey::from_seed(&decoded_secret_key)?))
        }
        Err(_) => {
            let keypair = ElGamalKeypair::new_rand();
            
            // Convert secret key to Vec<u8> and then to JSON, append to runtime_output.env file
            let secret_key_bytes = Vec::from(keypair.secret().as_bytes());
            let json_secret_key = serde_json::to_string(&secret_key_bytes)?;

            // Create runtime_output.env if it doesn't exist
            if !std::path::Path::new(RUNTIME_ENV_FILE_PATH).exists() {
                std::fs::File::create(RUNTIME_ENV_FILE_PATH)?;
            }

            // Open runtime_output.env file, create it if it does not exist
            let mut file = OpenOptions::new().append(true).create(true).open(RUNTIME_ENV_FILE_PATH)?;

            writeln!(file, "{}={}", variable_name, json_secret_key)?;

            Ok(keypair)
        },
    }
}

pub fn record_value<'a, T: serde::Serialize>(variable_name: &str, value: T) -> Result<T, Box<dyn Error>> {
    // Serialize the value to a JSON string
    let json_value = serde_json::to_string(&value)?;

    // Create runtime_output.env if it doesn't exist
    if !std::path::Path::new(RUNTIME_ENV_FILE_PATH).exists() {
        std::fs::File::create(RUNTIME_ENV_FILE_PATH)?;
    }

    // Read the existing runtime_output.env file content
    let mut content = std::fs::read_to_string(RUNTIME_ENV_FILE_PATH).unwrap_or_default();

    // Remove any existing line with the same variable name
    content = content
        .lines()
        .filter(|line| !line.starts_with(&format!("{}=", variable_name)))
        .collect::<Vec<&str>>()
        .join("\n");

    // Append the new variable value
    content.push_str(&format!("\n{}={}", variable_name, json_value));

    // Write the updated content back to the runtime_output.env file
    std::fs::write(RUNTIME_ENV_FILE_PATH, content)?;

    Ok(value)
}

pub fn load_value<T: serde::de::DeserializeOwned>(variable_name: &str) -> Result<T, Box<dyn Error>> {
    // First try to load from runtime_output.env
    if std::path::Path::new(RUNTIME_ENV_FILE_PATH).exists() {
        dotenvy::from_filename_override(RUNTIME_ENV_FILE_PATH).ok();
        if let Ok(env_value) = env::var(variable_name) {
            // Try to deserialize the JSON string to the object
            let value: Result<T, _> = serde_json::from_str(&env_value);
            
            // If deserialization succeeds, return the value
            if let Ok(val) = value {
                return Ok(val);
            }
            
            // Try to parse as a plain string
            let plain_value: Result<T, _> = serde_json::from_str(&format!("\"{}\"", env_value));
            if let Ok(val) = plain_value {
                return Ok(val);
            }
        }
    }
    
    // If not found in runtime_output.env, try the original .env
    dotenvy::from_filename_override(ENV_FILE_PATH).ok();
    
    // Get the environment variable
    let env_value = env::var(variable_name)?;

    // Try to deserialize the JSON string to the object
    let value: Result<T, _> = serde_json::from_str(&env_value);

    // If deserialization fails, try to parse it as a plain string
    match value {
        Ok(val) => Ok(val),
        Err(_) => {
            // Attempt to parse as a plain string or integer
            let plain_value: T = serde_json::from_str(&format!("\"{}\"", env_value))?;
            Ok(plain_value)
        }
    }
}

pub fn get_rpc_client() -> Result<RpcClient, Box<dyn Error>> {
    dotenvy::from_filename_override(ENV_FILE_PATH).ok();

    let client = RpcClient::new_with_commitment(
        String::from(env::var("RPC_URL")?),
        CommitmentConfig::confirmed(),
    );
    Ok(client)
}

pub fn get_non_blocking_rpc_client() -> Result<NonBlockingRpcClient, Box<dyn Error>> {  
    dotenvy::from_filename_override(ENV_FILE_PATH).ok();

    let client = NonBlockingRpcClient::new_with_commitment(
        String::from(env::var("RPC_URL")?),
        CommitmentConfig::confirmed(),
    );
    Ok(client)
}

/// Spawns a blocking task to generate both AeKey and ElGamalKeypair from a given signer.
/// This utility function helps avoid Tokio runtime conflicts by isolating blocking operations.
pub async fn tokio_spawn_blocking_turnkey_signer_keys(
    private_key_id_env: &str,
    public_key_env: &str,
) -> Result<(Box<dyn Signer + Send>, AeKey, ElGamalKeypair), String> {
    let private_key_id = private_key_id_env.to_string();
    let public_key = public_key_env.to_string();
    
    tokio::task::spawn_blocking(move || -> Result<(Box<dyn Signer + Send>, AeKey, ElGamalKeypair), String> {
        let signer = get_turnkey_signer(&private_key_id, &public_key)
            .map_err(|e| e.to_string())?;
        
        let elgamal_keypair = ElGamalKeypair::new_from_signer(&signer, &signer.pubkey().to_bytes())
            .map_err(|e| e.to_string())?;
        
        let aes_key = AeKey::new_from_signer(&signer, &signer.pubkey().to_bytes())
            .map_err(|e| e.to_string())?;
        
        Ok((signer, aes_key, elgamal_keypair))
    })
    .await
    .map_err(|e| e.to_string())?
}

pub fn get_turnkey_signers_from_env(
    private_key_id_env: &str,
    public_key_env: &str,
) -> Result<Box<dyn Signer + Send>, String> {
    let private_key_id = private_key_id_env.to_string();
    let public_key = public_key_env.to_string();
    
    let signer = get_turnkey_signer(&private_key_id, &public_key)
        .map_err(|e| e.to_string())?;
    
    Ok(signer)
}

pub async fn get_gcp_signer_from_env(
    resource_name: &str,
) -> Result<GcpSigner, Box<dyn Error>> {
    dotenvy::from_filename_override(ENV_FILE_PATH).ok();

    let signer = GcpSigner::new(resource_name.to_string()).await?;
    Ok(signer)
}

pub async fn run_with_retry<F, Fut>(
    max_retries: usize,
    operation: F,
) -> Result<(), Box<dyn Error>>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<(), Box<dyn Error>>>,
{
    for attempt in 1..=max_retries {
        println!("Attempt {} of {}", attempt, max_retries);
        match operation().await {
            Ok(_) => return Ok(()),
            Err(e) => {
                println!("Error: {}. Retrying...", e);
                if attempt == max_retries {
                    return Err(e);
                }
            }
        }
    }
    Ok(())
}

pub fn print_transaction_url(pre_text: &str, signature: &str) {
    const SOLANA_EXPLORER_URL: &str = "https://explorer.solana.com/tx/";

    let cluster = match env::var("RPC_URL").unwrap_or_default() {
        url if url.contains("devnet") => "?cluster=devnet",
        url if url.contains("testnet") => "?cluster=testnet",
        _ => "",
    };

    println!(
        "\n{}: {}{}{}",
        pre_text,
        SOLANA_EXPLORER_URL,
        signature,
        cluster
    );
}
