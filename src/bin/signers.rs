use clap::{value_t_or_exit, App, Arg};
use solana_clap_utils::keypair::signer_from_path;
use solana_clap_utils::offline::OfflineArgs;


#[tokio::main]
#[allow(dead_code)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let clap_app = App::new("my-program")
        // The argument we'll parse as a signer "path"
        .arg(
            Arg::with_name("keypair")
                .required(true)
                .help("The default signer"),
        )
        .offline_args();

    let clap_matches = clap_app.get_matches();
    let keypair_str = value_t_or_exit!(clap_matches, "keypair", String);
    let mut wallet_manager = None;
    let signer = signer_from_path(&clap_matches, &keypair_str, "keypair", &mut wallet_manager)?;

    print!("Signer pubkey: {}\n", signer.pubkey());
    Ok(())
}

use solana_remote_wallet::*;

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