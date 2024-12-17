# Block Explorers

## User Stories

### As a confidential token holder, I need a way to view my balance on a block explorer.
Block explorers provide a decrypt button where balance is displayed.
Interfacing with the button prompts the user to sign a message, whose [signature is used as the decryption key](https://github.com/anza-xyz/agave/blob/d11072e4e00cb3a8009f62b3bddcec79069f970a/zk-sdk/src/encryption/elgamal.rs#L209).  

Upon signing, the encrypted balance is replaced with the decrypted amount.

Requirements:
- Derived decrpytion keys must correspond to either the sender, receiver, or auditor ciphertext.
- Both ElGamal & EAS key derivations must use the owner's account public key as the seed.
- Errors must be displayed to the user if the decryption key is invalid.