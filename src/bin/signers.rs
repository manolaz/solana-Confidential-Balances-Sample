use solana_remote_wallet::*;
use solana_sdk::{
    hash::Hash, 
    message, 
    native_token::LAMPORTS_PER_SOL, 
    signer::Signer, 
    system_instruction, 
    transaction::Transaction
};

#[tokio::main]
#[allow(dead_code)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
      
    let signer = load_signer_from_ledger("wallet0", false)?;
    println!("Signer: {:?}", signer.pubkey);

    // Transfer SOL to another account
    let to = signer.pubkey();
    let from = signer.pubkey();
    let amount = LAMPORTS_PER_SOL * 42;
    let recent_blockhash = Hash::default();
    let ix = system_instruction::transfer(&from, &to, amount);
    let message = message::Message::new(&[ix], Some(&from));
    let mut transaction = Transaction::new_unsigned(
        message,
    );

    println!("Txn Signature START");
    transaction.try_sign(&[&signer], recent_blockhash)?;
    println!("Txn Signature END: {:?}", transaction.signatures[0]);


    println!("Message Signature START");
    let message = b"hello, world!";
    let sig = signer.try_sign_message(message)?; // Error: Protocol("Ledger received invalid Solana message")
    println!("Message Signature END: {:?}", sig);

    Ok(())
}



pub fn load_signer_from_ledger(keypair_name:&str, confirm_key : bool) -> Result< remote_keypair::RemoteKeypair, Box<dyn std::error::Error>> {
    let wallet_manager = remote_wallet::maybe_wallet_manager()?.unwrap();
    
    let locator = locator::Locator::new_from_path("usb://ledger")?;

    Ok(remote_keypair::generate_remote_keypair(
        locator,
        solana_sdk::derivation_path::DerivationPath::default(),
        &wallet_manager,
        confirm_key,
        keypair_name,
    )?)
}