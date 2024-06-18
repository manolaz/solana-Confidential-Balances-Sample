# Setup
At the time of writing, SPL repo contains an undeployed version of Token22 with active developments on confidential transfer extension. To run the sample, and the known [CLI example](https://github.com/solana-labs/solana-program-library/blob/d9a6ee8db65167098b654b300ac23abc08fd8a7d/token/cli/examples/confidential-transfer.sh#L1), you will need to [build the Token22 program from source and deploy it on the solana-test-validator](https://solana.stackexchange.com/questions/10062/errors-when-trying-out-confidential-transfer-token-extension-on-solana-test-vali).

1. `git clone https://github.com/solana-labs/solana-program-library.git`  

1. `cargo build-sbf`

1. `solana-test-validator -r --bpf-program TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb target/deploy/spl_token_2022.so`

1. `cargo run --bin main`