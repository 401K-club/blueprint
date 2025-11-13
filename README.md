# Jimmi
This blueprint implements a marketplace for a coin called `Jimmi` against XRD.  
The movement of the `Jimmi` coin is restricted: coins can only be sent to/from the component, it's not possible to transfer them amoung users or towards other components.  
Both buy and sell operations are subjected to two fees: dividends and jackpot.  
When a user sells his `Jimmi` he is accrued part of the collected dividends from the buy operation to the sell one.  

## Buy
```
CALL_METHOD
    Address("<ACCOUNT_ADDRESS>")
    "withdraw"
    Address("<XRD_ADDRESS>")
    Decimal("<XRD_AMOUNT>")
;
TAKE_ALL_FROM_WORKTOP
    Address("<XRD_ADDRESS>")
    Bucket("xrd")
;
CALL_METHOD
    Address("<COMPONENT>")
    "buy"
    Bucket("xrd")
;
TAKE_ALL_FROM_WORKTOP
    Address("<JIMMI_ADDRESS>")
    Bucket("jimmi")
;
TAKE_ALL_FROM_WORKTOP
    Address("<DEPOSIT_BADGE_ADDRESS>")
    Bucket("deposit_badge")
;
CREATE_PROOF_FROM_BUCKET_OF_ALL
    Bucket("deposit_badge")
    Proof("deposit_badge")
;
PUSH_TO_AUTH_ZONE
    Proof("deposit_badge")
;
CALL_METHOD
    Address("<ACCOUNT_ADDRESS>")
    "deposit"
    Bucket("jimmi")
;
POP_FROM_AUTH_ZONE
    Proof("deposit_badge1")
;
DROP_PROOF
    Proof("deposit_badge1")
;
CALL_METHOD
    Address("<COMPONENT>")
    "post_buy"
    Address("<ACCOUNT_ADDRESS>")
    Bucket("deposit_badge")
;
```

`<ACCOUNT_ADDRESS>` user's account address.  
`<XRD_ADDRESS>` XRD resource address.  
`<XRD_AMOUNT>` the amount of XRD used to buy `Jimmi`.  
`<COMPONENT>` Jimmi component address.  
`<JIMMI_ADDRESS>` Jimmi coin resource address.  
`<DEPOSIT_BADGE_ADDRESS>` deposit badge resource address.  

This method emits a `BuyEvent` event containing:  
- `account`: the address of the buyer account.  
- `price`: bought price (dividends and jackpot excluded).  
- `bought_amount`: the number of bought Jimmi.  
- `total_dividends_amount`: total amount of not assigned dividends in the component.  
- `current_jackpot_amount`: current amount of the jackpot.  
- `global_dividends_per_jimmi`: amount of dividends per Jimmi.  
- `buyer_dividends_per_jimmi`: weighted average of `global_dividends_per_jimmi` during buy operations from this user.  
- `ath`: Jimmi ATH since the last jackpot distribution.  
- `buyer_accrued_jackpot`: Past jackpots amount accrued to the buyer.  

## Sell
```
CALL_METHOD
    Address("<COMPONENT>")
    "pre_sell"
;
TAKE_ALL_FROM_WORKTOP
    Address("<WITHDRAW_BADGE_ADDRESS>")
    Bucket("withdraw_badge")
;
CREATE_PROOF_FROM_BUCKET_OF_ALL
    Bucket("withdraw_badge")
    Proof("withdraw_badge")
;
PUSH_TO_AUTH_ZONE
    Proof("withdraw_badge")
;
CALL_METHOD
    Address("<ACCOUNT_ADDRESS>")
    "withdraw"
    Address("<JIMMI_ADDRESS>")
    Decimal("<JIMMI_AMOUNT>")
;
POP_FROM_AUTH_ZONE
    Proof("withdraw_badge1")
;
DROP_PROOF
    Proof("withdraw_badge1")
;
TAKE_ALL_FROM_WORKTOP
    Address("<JIMMI_ADDRESS>")
    Bucket("jimmi")
;
CALL_METHOD
    Address("<COMPONENT>")
    "sell"
    Bucket("jimmi")
    Address("<ACCOUNT_ADDRESS>")
    Bucket("withdraw_badge")
;
CALL_METHOD
    Address("<ACCOUNT_ADDRESS>")
    "deposit_batch"
    Expression("ENTIRE_WORKTOP")
;
```

`<ACCOUNT_ADDRESS>` user's account address.  
`<COMPONENT>` Jimmi component address.  
`<JIMMI_ADDRESS>` Jimmi coin resource address.  
`<JIMMI_AMOUNT>` The amount of Jimmi to sell.  
`<WITHDRAW_BADGE_ADDRESS>` withdraw badge resource address.  

This method emits a `SellEvent` event containing:  
- `account`: the address of the seller account.  
- `price`: sold price (dividends and jackpot excluded).  
- `sold_amount`: the number of sold Jimmi.  
- `total_dividends_amount`: total amount of not assigned dividends in the component.  
- `current_jackpot_amount`: current amount of the jackpot.  
- `global_dividends_per_jimmi`: amount of dividends per Jimmi.  
- `buyer_total_accrued_dividends`: the pending accrued dividends from all the sell operations from this account (zero if `<RECEIVE_DIVIDENDS>` is `true`).  
- `buyer_accrued_jackpot`: past jackpots amount accrued to the seller.  

# Airdrop
```
CALL_METHOD
    Address("<ACCOUNT_ADDRESS>")
    "withdraw"
    Address("<XRD_ADDRESS>")
    Decimal("<XRD_AMOUNT>")
;
TAKE_ALL_FROM_WORKTOP
    Address("<XRD_ADDRESS>")
    Bucket("xrd")
;
CALL_METHOD
    Address("<COMPONENT>")
    "airdrop"
    Bucket("xrd")
    Map<Address, Decimal>(
        Address("<RECIPIENT_ADDRESS>") => Decimal("<SHARE>"),
        ...
    )
;
```

`<ACCOUNT_ADDRESS>` user's account address.  
`<XRD_ADDRESS>` XRD resource address.  
`<XRD_AMOUNT>` the amount of XRD used to buy `Jimmi`.  
`<COMPONENT>` Jimmi component address.  
`<RECIPIENT_ADDRESS>`: account address of one of the recipients.  
`<SHARE>`: share of jimmi coins to send to this recipient. The sum of all shares must be 1.  

This method emits a `BuyEvent` event (see buy operation) for each recipient.  

# Withdraw dividends
```
CALL_METHOD
    Address("<COMPONENT>")
    "withdraw_dividends"
    Address("<ACCOUNT_ADDRESS>")
;
CALL_METHOD
    Address("<ACCOUNT_ADDRESS>")
    "deposit_batch"
    Expression("ENTIRE_WORKTOP")
;
```

`<COMPONENT>` Jimmi component address.  
`<ACCOUNT_ADDRESS>` user's account address.  

This method emits a `WithdrawDividendsEvent` event containing:  
- `account`: the address of the account withdrawing his dividends.  
- `withdrawn_dividends`: the amount of dividends withdrawn in this operation.  
- `withdrawn_jackpot`: amount of withdrawn jackpot.  

