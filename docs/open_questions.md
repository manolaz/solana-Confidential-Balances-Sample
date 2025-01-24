# Open Questions
- How does a new global auditor decrypt confidential transfers from prior global auditor's key?
  - We should define expectations for auditor transference.
- Is the range proof at the limit of the transaction size?
  - I couldn't add a third-party signer to the range proof txn without triggering the txn size limit.
  - Sender->Receiver, where Sender pays the fee is a natural use-case if it must be so.
  - Using proof Record for proofs solved this problem.

- How does a sender create a receiver's token account when the receiver requires setting up a decryption key?

- What does block explorer UX look like for this product?

- Can one user create a confidential token account for another user?

- Considering the encryption flows, how can PDA's interface with confidential tokens? This is a use case for escrows and other DeFi protocols.

- Can we use Jito bundles to batch confidential transfer transactions?

- How can senders & receivers verify confidential transfer transactions when associated context state accounts are closed?

- How can a sender be prompted to sign only once for a confidential transfer (which involves multiple signatures)?

# Issuer
1. Auditor Rotation & Regulatory Oversight  
   - How frequently does Issuer anticipate rotating auditors or key custodians?  
   - What internal policies or processes exist for replacing or revoking an auditor’s key?  
   - Does Issuer require a fully on-chain approach, or would an off-chain MPC or hybrid MPC model be preferable for their compliance needs?  
   - What kind of auditable logs, records, or notifications would be required each time an auditor rotates?  

2. GDPR & Data Privacy  
   - What personal data does Issuer need to encrypt or obscure in transaction records to meet GDPR requirements?  
   - Are there constraints on storing or transmitting encrypted data that is still technically “personal” under GDPR?  
   - How might Issuer handle user requests for data deletion or export under GDPR in a system where confidential transactions are stored on-chain?  

3. AML, KYC, and Risk Controls  
   - What specific AML workflows must be integrated into confidential transfers (e.g., flagged addresses, transaction monitoring)?  
   - Does Issuer require real-time or post-transaction auditing for suspicious activity?  
   - How can Issuer’s existing KYC checks be extended to participants in Solana-based confidential transfers?  

4. Token Issuance & Governance  
   - How much control does Issuer want over token issuance, freezing, or clawbacks under certain conditions (e.g., fraud)?  
   - Would Issuer use a single global auditor key or prefer multiple auditor keys for different jurisdictions?  
   - Does Issuer require a nuanced approach where certain transfers remain publicly visible while others are confidential?  

5. Operational Scale & Performance
   - How many transactions per second does an Issuer anticipate if confidential transfers are enabled?  
   - Are there concerns about transaction sizes, or the additional overhead of proof accounts, for high-volume usage?  
   - What internal performance metrics or service-level agreements (SLAs) does Issuer expect to meet?  

6. User Experience & Integration  
   - How does Issuer envision integrating confidential transfers into its existing user workflows (e.g., Issuer web interface, mobile app)?  
   - Are there preferences for how users are prompted to authorize or decrypt confidential balances?  
   - What fallback or “fail-open” approaches does Issuer consider acceptable if encryption keys become inaccessible?  

7. Ecosystem & Partner Compliance  
   - How does Issuer handle partner wallets, block explorers, or DeFi integrations that may reveal partial transaction details?  
   - What expectations does Issuer have for ecosystem participants (exchanges, custodians, analytics firms) to comply with confidentiality requirements?  

8. Future Upgrades & Roadmap  
   - How critical is the ability to upgrade the token or the mint account to adopt new privacy features or comply with changing regulations?  
   - What roadmap does Issuer foresee for advancing confidentiality (e.g., zero-knowledge proofs or MPC enhancements) over time?  

9. Liability & Risk Mitigation  
   - How does Issuer plan to handle liability if an auditor’s key is compromised—are there fallback measures (like re-issuing tokens)?  
   - Do existing insurance frameworks or risk models apply to losses or incidents related to confidential transfer edge cases?  

10. Reporting & Auditing Requirements  
   - Which auditing standards must Issuer conform to (e.g., SOC2, ISO 27001, or departmental audits for regulated stablecoins)?  
   - Under what conditions must transaction data be revealed to regulatory bodies on short notice?  
   - How detailed must confidential transaction logs be for annual or quarterly finance reviews?

These questions will help clarify Issuer’s operational, compliance, and user experience constraints—shaping how the reference implementations in this project can be adapted for a successful $PYUSD launch with confidential transfers on Solana.