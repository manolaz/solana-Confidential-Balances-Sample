# Open Questions
- How does a new global auditor decrypt confidential transfers from prior global auditor's key?
  - We should define expectations for auditor transference.
- Is the range proof at the limit of the transaction size?
  - I couldn't add a third-party signer to the range proof txn without triggering the txn size limit.
  - Sender->Receiver, where Sender pays the fee is a natural use-case if it must be so.
  - Using proof Record for proofs solved this problem.

- How does a sender create a receiver's token account when the receiver requires setting up a decryption key?

- What does block explorer UX look like for this product?