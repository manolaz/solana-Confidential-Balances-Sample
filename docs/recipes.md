# Recipes

## Table of Contents
- [Basic Transfer](#basic-transfer)
- [Confidential MintBurn Transfer](#confidential-mintburn-transfer)


## [Basic Transfer](../recipes/src/lib.rs#L43)
### Scenario:
- Public mint account (without confidential mint/burn extension).  
- Alice makes an offchain request to the Token Issuer (mint authority) for confidentially redeeming cUSD stablecoins. 
- Token Issuer delivers funds to Alice's token account as confidential transfer.

### Notes:
- The mint account mints/burns publicly, and requires Deposit & Apply instructions prior to Confidential Transfer.
    - The Token Issuer must have their own token account to receive the minted tokens.
- Due to public mint/burn, the transfer is only partially confidential.
```mermaid
sequenceDiagram
    participant Issuer as Token Issuer's Wallet
    participant Validator as System Program
    participant Token22 as Token22 Program
    participant Mint as cUSD Mint Account
    participant ElGamal as ElGamal Proof Program
    participant IssuerATA as Token Issuer's Token Account
    participant AliceATA as Alice's Token Account
    participant Alice as Alice's Wallet
    
    Note over Issuer,Alice: Step 1: Create Wallet Keypairs
    Issuer->>Validator: Create wallet keypair & allocate account
    Alice->>Validator: Create wallet keypair & allocate account
    
    Note over Issuer,Mint: Step 2: Create Mint Account
    Issuer->>Validator: Create mint account (Mint Authority: Token Issuer)
    Validator-->>Mint: Create
    Issuer->>Token22: Initialize Mint
    Issuer->>Token22: Enable confidential transfer extension on Mint
    
    Note over Issuer,IssuerATA: Step 3: Create Sender Token Account
    Issuer->>Issuer: Generate ElGamal keypair
    Issuer->>Issuer: Generate AES key
    Issuer->>IssuerATA: Create Associated Token Account (ATA)
    Issuer->>Token22: Configure confidential extension
    Token22-->>IssuerATA: Configure
    
    Note over Issuer,IssuerATA: Step 4: Mint Tokens
    Issuer->>Token22: Mint tokens
    Token22-->>IssuerATA: Credit minted amount
    
    Note over Issuer,IssuerATA: Step 5: Deposit Tokens
    Issuer->>Token22: Deposit to pending confidential balance
    Token22-->>IssuerATA: Decrease (non-confidential) public token balance
    Token22-->>IssuerATA: Increase pending balance
    
    Note over Issuer,IssuerATA: Step 6: Apply Sender's Pending Balance
    Issuer->>Token22: Apply pending to available balance
    Token22-->>IssuerATA: Decrease pending balance.
    Token22-->>IssuerATA: Increase available/confidential balance.
    
    Note over Alice,AliceATA: Step 7: Create Recipient Token Account
    Alice->>Alice: Generate ElGamal keypair
    Alice->>Alice: Generate AES key
    Alice->>AliceATA: Create Associated Token Account (ATA)
    Alice->>Token22: Configure confidential extension
    Token22-->>AliceATA: Configure
    
    Note over Issuer,AliceATA: Step 8: Transfer with Proofs
    Issuer->>Issuer: Generate proof data for transfer
    Issuer->>+Issuer: Generate proof accounts
    Issuer->>Validator: Create equality proof
    Issuer->>Validator: Create ciphertext validity proof
    Issuer->>-Validator: Create range proof
    
    Issuer->>+Issuer: Execute confidential transfer
    Issuer->>ElGamal: Verify equality proof
    Issuer->>ElGamal: Verify ciphertext proof
    Issuer->>ElGamal: Verify range proof
    Issuer->>-Token22: Execute confidential transfer

    Token22-->>IssuerATA: Debit encrypted amount
    Token22-->>AliceATA: Credit encrypted amount
    
    Issuer->>+Issuer: Close proof accounts
    Issuer->>ElGamal: Close equality proof
    Issuer->>ElGamal: Close ciphertext validity proof
    Issuer->>-ElGamal: Close range proof

    Note over Alice,AliceATA: Step 9: Apply Recipient's Pending Balance
    Alice->>Token22: Apply pending to available balance
    Token22-->>AliceATA: Decrease pending balance
    Token22-->>AliceATA: Increase available/confidential balance
    
    Note over Alice,AliceATA: Step 10: Withdraw Tokens (Optional)
    Alice->>Alice: Generate proof data for withdraw
    Alice->>+Alice: Generate proof accounts
    Alice->>Validator: Create range proof account
    Alice->>-Validator: Create equality proof acount

    Alice->>+Alice: Execute withdrawal
    Alice->>ElGamal: Verify range
    Alice->>ElGamal: Verify equality
    Alice->>-Token22: Withdraw tokens from confidential balance
    Token22-->>AliceATA: Decrease available/confidential balance
    Token22-->>AliceATA: Increase (non-confidential) public token balance
    Alice->>+Alice: Close proof accounts
    Alice->>ElGamal: Close range proof account
    Alice->>-ElGamal: Close equality proof account
```

## [Confidential MintBurn Transfer](../recipes/src/lib.rs#L18)
Scenario:
- Confidential mint account (using confidential mint/burn extension).  
```mermaid
sequenceDiagram
Note over WIP: Work in progess
```