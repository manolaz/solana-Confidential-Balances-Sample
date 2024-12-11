# Auditor Rotation

Auditor rotation is the process of changing the global auditor of a token.
Rotation capabilities depend on the chosen solution:

## On-chain with Key Derivation
Steps:
1. Mint account is initialized with an auditor keypair.
1. Auditor1 sends a copy of auditor keypair to Auditor2.
1. Auditor2 derives a new keypair from the original.
1. Mint authority updates the mint account with Auditor2's pubkey.

Limitations:
- Cannot revoke Auditor1 access. Auditor1 keeps a copy of the keypair.
- Compromised keypair irrevocably exposes associated confidential transfers.
- Auditor1 knows the original seed, they can brute force Auditor2's derivation seed.
- Every auditor rotation increments the total derivations to track.
  - This is a problem for frequently rotating auditor sets.
  - Requires separate tooling for associating transfers with the corresponding derivation.
  
```mermaid
flowchart LR
 subgraph s1["Mint Account"]
        n1["Auditor Pubkey"]
  end
    n2["Auditor1 Keypair"] -. Init .-> n1
    n2 -- derive(seed) --> n3["Auditor2 Keypair"]
    n3 -- Update --> n1

    %%n2@{ icon: "azure:key-vaults", pos: "b"}
    %%n3@{ icon: "azure:key-vaults", pos: "b"}
    style n2 stroke:#757575
    style n3 stroke:#DD6D00
```



## Off-chain MPC
1. MPC is initialized with 2-of-2 quorum:
    - ShareA: MPC facilitator service (neutral party).
    - ShareB: Auditor1.
1. Mint account is initialized with MPC public key as auditor pubkey.
1. Auditor1 invokes MPC reshare to add Auditor2 to the MPC.
  1. MPC facilitator service gathers ShareA and ShareB.
  1. MPC facilitator service generates ShareC for Auditor2.
  1. MPC facilitator service invalidates ShareB.

Benefits:
- Enables auditor revocation.
- Allows for multiple simultaneous auditors.
- Mitigates risk of compromised auditor key.
- Keeps a consistent auditor keypair regardless of the auditor set.
- Cost-effective for frequent auditor rotations.

Limitations:
- Cannot compartmentalize auditor access to a subset of confidential transfers.
- Requires third-party MPC facilitator service.
- Lacks verifiability
    - Can't tell when a new auditor is added to the MPC.
    - Can't identify an individual auditor's key.
- Trusted setup is required for MPC.

```mermaid
flowchart LR
 subgraph s2["Mint Account"]
        n8["Auditor Pubkey"]
  end 
 subgraph s4["Auditor2"]
        n6["ShareC"]
  end  subgraph s3["Auditor1"]
        n5["<strike>ShareB</strike>"]
  end


 subgraph s1["Facilitator"]
        n7["ShareA"]
        s5["MPC"]
  end

    s3 -- [1] Rotate Auditor --> s1
    s1 -- [2] Create new share --> n6
    s1 -- [3] Invalidate old share --> n5
    s5 ---> n8

    style n6 stroke-width:4px,stroke-dasharray: 5
```

## Hybrid MPC
Assumes deployment of an "Auditor Rotation Program" (ARP)
### Setup
1. MPC is initialized with 2-of-2 quorum:
    - ShareA: MPC facilitator service (neutral party).
    - ShareB: Auditor1.
1. MPC facilitator service listens for transactions to ARP.
1. Mint account is initialized with MPC public key as auditor pubkey.
```mermaid
flowchart LR
 subgraph s5["MPC Facilitator"]
        n7["ShareA"]
        s1["MPC"]
  end
 subgraph s2["Mint Account"]
        n8["Auditor Pubkey"]
  end
 subgraph s3["Auditor1"]
        n5["ShareB"]
  end
    s1 -- [1a] Create --> n7
    s1 -- [1b] Create --> n5
    s5 -- [2] Listen for Rotate instruction --> n9["Auditor<br>Rotation<br>Program"]
    n10["Mint Authority"] -- [3] Init(MPC) --> s2

    n10@{ shape: rect}
```

### Rotation
1. Mint authority sends Rotate Auditor instruction to ARP.
    - Contains new Auditor2's MPC ID.
1. ARP generates a Pending Rotation PDA.
1. MPC facilitator reacts to ARP transaction.
    1. MPC facilitator gathers ShareA and ShareB.
    1. MPC facilitator generates ShareC for Auditor2.
    1. MPC facilitator invalidates ShareB.
1. MPC facilitator submits a "Confirm Rotation" transaction to ARP.
    1. ARP verifies confirmation is valid.
    1. ARP closes the Pending Rotation PDA.

```mermaid
flowchart LR
 subgraph s5["MPC Facilitator"]
        n7["ShareA"]
        s1["MPC"]
  end
 subgraph s2["Mint Account"]
        n8["Auditor Pubkey"]
  end
 subgraph s4["Auditor2"]
        n6["ShareC"]
  end
 subgraph s3["Auditor1"]
        n5["<strike>ShareB</strike>"]
  end
 subgraph s6["PDA"]
        n10["Pending <br>Rotation<br>Signature"]
  end
 subgraph s7["Transaction"]
        n12["Rotate"]
  end
 subgraph s8["Transaction"]
        n13["Confirm Rotation"]
  end
    s1 ---> n8
    n9["Auditor<br>Rotation<br>Program"] -- [2] Create pending record --> s6
    n11["Mint Authority"] --> s7
    s7 -- [1] Rotate Ixn --> n9
    s5 o-. Listening for Rotate instruction .-o n9
    s5 -- [3b] Invalidate --> n5
    s5 -- [3a] Create --> n6
    n9 -- [5] Close --> s6
    s5 --> s8
    s8 -- [4] Confirm txn --> n9

    n9@{ shape: rect}
    n11@{ shape: rect}
    style n6 stroke-width:4px,stroke-dasharray: 5
```

## Custodial
This is a solution with an opaque custodian.
The custodian is trusted to:
- Update mint account with new auditor keypair.
- Facilitate providing correct decrpytion keypair for each auditor era.
- Provide authentication mechanism for each auditor.
- Determine how to custody the underlying keypair.

There are many ways to implement this solution.
It's up to the custodian to determine the best approach.