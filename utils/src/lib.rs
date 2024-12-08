use solana_client::nonblocking::rpc_client::RpcClient as NonBlockingRpcClient;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signer::keypair::Keypair;
use solana_zk_sdk::encryption::elgamal::{ElGamalKeypair, ElGamalSecretKey};
use std::env;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;

const ENV_FILE_PATH: &str = "../.env";
const RPC_URL: &str = "http://127.0.0.1:8899";

// Get or create a keypair from an .env file
pub fn get_or_create_keypair(variable_name: &str) -> Result<Keypair, Box<dyn Error>> {
    dotenv::dotenv().ok();

    match env::var(variable_name) {
        Ok(secret_key_string) => {
            // Fallback to JSON format
            let decoded_secret_key: Vec<u8> = serde_json::from_str(&secret_key_string)?;
            Ok(Keypair::from_bytes(&decoded_secret_key)?)
        }
        Err(_) => {
            // Create a new keypair if the environment variable is not found
            let keypair = Keypair::new();

            // Convert secret key to Vec<u8> and then to JSON, append to .env file
            let secret_key_bytes = Vec::from(keypair.to_bytes());
            let json_secret_key = serde_json::to_string(&secret_key_bytes)?;

            // Open .env file, create it if it does not exist
            let mut file = OpenOptions::new().append(true).create(true).open(ENV_FILE_PATH)?;

            writeln!(file, "{}={}", variable_name, json_secret_key)?;

            Ok(keypair)
        }
    }
}

pub fn get_or_create_keypair_elgamal(variable_name: &str) -> Result<ElGamalKeypair, Box<dyn Error>> {
    dotenv::dotenv().ok();

    match env::var(variable_name) {
        Ok(secret_key_string) => {
            let decoded_secret_key: Vec<u8> = serde_json::from_str(&secret_key_string)?;
            Ok(ElGamalKeypair::new(ElGamalSecretKey::from_seed(&decoded_secret_key)?))
        }
        Err(_) => {
            let keypair = ElGamalKeypair::new_rand();
            
            // Convert secret key to Vec<u8> and then to JSON, append to .env file
            let secret_key_bytes = Vec::from(keypair.secret().as_bytes());
            let json_secret_key = serde_json::to_string(&secret_key_bytes)?;

            // Open .env file, create it if it does not exist
            let mut file = OpenOptions::new().append(true).create(true).open(ENV_FILE_PATH)?;

            writeln!(file, "{}={}", variable_name, json_secret_key)?;

            Ok(keypair)
        },
    }
}

pub fn record_value<'a, T: serde::Serialize>(variable_name: &str, value: T) -> Result<T, Box<dyn Error>> {
    dotenv::dotenv().ok();

    // Serialize the value to a JSON string
    let json_value = serde_json::to_string(&value)?;

    // Read the existing .env file content
    let mut content = std::fs::read_to_string(ENV_FILE_PATH).unwrap_or_default();

    // Remove any existing line with the same variable name
    content = content
        .lines()
        .filter(|line| !line.starts_with(&format!("{}=", variable_name)))
        .collect::<Vec<&str>>()
        .join("\n");

    // Append the new variable value
    content.push_str(&format!("\n{}={}", variable_name, json_value));

    // Write the updated content back to the .env file
    std::fs::write(ENV_FILE_PATH, content)?;

    Ok(value)
}

pub fn load_value<T: serde::de::DeserializeOwned>(variable_name: &str) -> Result<T, Box<dyn Error>> {
    dotenv::dotenv().ok();

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
    let client = RpcClient::new_with_commitment(
        String::from(RPC_URL),
        CommitmentConfig::confirmed(),
    );
    Ok(client)
}

pub fn get_non_blocking_rpc_client() -> Result<NonBlockingRpcClient, Box<dyn Error>> {
    let client = NonBlockingRpcClient::new_with_commitment(
        String::from(RPC_URL),
        CommitmentConfig::confirmed(),
    );
    Ok(client)
}