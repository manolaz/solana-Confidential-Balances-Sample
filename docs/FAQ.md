# FAQ

## Can an initialized mint retroactively add confidential transfers extension?
No. (Todo: present alternative)

## How to confidentialize a Confidential Transfers mint extension without also adding the Confidential MintBurn extension?
Have the token issuer mint, deposit, and apply aggregate/bulk tokens in advance. Then use an offchain method to confidentially transfer partial amounts to the intended recipient. This is sort of a quasi-mint-burn.

## What is the minimum number of CU's, accounts, and transactions needed to make a Confidential Transfer?
It depends on the the UX flow most relevant to the application. (Todo: point toward the most compact recipe)