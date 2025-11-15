# 401k Club
This blueprint implements a marketplace for a coin called `401k` against XRD.  
The movement of the `401k` coin is restricted: coins can only be sent to/from the component, it's not possible to transfer them amoung users or towards other components.  
Both buy and sell operations are subjected to two fees: dividends and jackpot.  
When a user owns `401k` he is accrued part of the collected dividends from the buy and sell operations performed by all users.  

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
    Address("<401K_ADDRESS>")
    Bucket("401k")
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
    Bucket("401k")
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
`<XRD_AMOUNT>` the amount of XRD used to buy `401k`.  
`<COMPONENT>` 401kClub component address.  
`<401K_ADDRESS>` 401k coin resource address.  
`<DEPOSIT_BADGE_ADDRESS>` deposit badge resource address.  

This method emits a `BuyEvent` event containing:  
- `account`: the address of the buyer account.  
- `price`: bought price (dividends and jackpot excluded).  
- `bought_amount`: the number of bought 401k.  
- `current_jackpot_amount`: current amount of the jackpot.  
- `global_dividends_per_401k`: amount of dividends per 401k.  
- `ath`: 401k ATH since the last jackpot distribution.  
- `buyer_total_accrued_dividends`: Dividends accrued to the buyer so far.  
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
    Address("<401K_ADDRESS>")
    Decimal("<401K_AMOUNT>")
;
POP_FROM_AUTH_ZONE
    Proof("withdraw_badge1")
;
DROP_PROOF
    Proof("withdraw_badge1")
;
TAKE_ALL_FROM_WORKTOP
    Address("<401K_ADDRESS>")
    Bucket("401k")
;
CALL_METHOD
    Address("<COMPONENT>")
    "sell"
    Bucket("401k")
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
`<COMPONENT>` 401kClub component address.  
`<401K_ADDRESS>` 401k coin resource address.  
`<401K_AMOUNT>` The amount of 401k to sell.  
`<WITHDRAW_BADGE_ADDRESS>` withdraw badge resource address.  

This method emits a `SellEvent` event containing:  
- `account`: the address of the seller account.  
- `price`: sold price (dividends and jackpot excluded).  
- `sold_amount`: the number of sold 401k.  
- `total_dividends_amount`: total amount of not assigned dividends in the component.  
- `current_jackpot_amount`: current amount of the jackpot.  
- `global_dividends_per_401k`: amount of dividends per 401k.  
- `seller_total_accrued_dividends`: the dividends accrued to the seller.  
- `seller_accrued_jackpot`: past jackpots amount accrued to the seller.  

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
`<XRD_AMOUNT>` the amount of XRD used to buy `401k`.  
`<COMPONENT>` 401kClub component address.  
`<RECIPIENT_ADDRESS>`: account address of one of the recipients.  
`<SHARE>`: share of 401k coins to send to this recipient. The sum of all shares must be 1.  

This method emits a `BuyEvent` event (see buy operation) for each recipient and a single `AirdropCompletedEvent` containing:  
- `global_dividends_per_401k`: amount of dividends per 401k (this is the final value, the one in the `BuyEvents` is obsolete).  

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

`<COMPONENT>` 401kClub component address.  
`<ACCOUNT_ADDRESS>` user's account address.  

This method emits a `WithdrawDividendsEvent` event containing:  
- `account`: the address of the account withdrawing his dividends.  
- `withdrawn_dividends`: the amount of dividends withdrawn in this operation.  
- `withdrawn_jackpot`: amount of withdrawn jackpot.  

