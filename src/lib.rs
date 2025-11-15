use scrypto::prelude::*;

/* NonFungibleData of the badge needed to deposit 401k in a Vault.
 * A badge is minted by the buy method, it must be burned by the post_sell
 * method.
 */
#[derive(ScryptoSbor, NonFungibleData)]
struct DepositBadge {
    // Amount of 401k bought
    bought_amount: Decimal,
    // Price the 401k were bought at
    price: Decimal,
    // Amount of dividends paid in the current buy operation
    dividends_amount: Decimal,
}

/* NonFungibleData of the badge needed to withdraw 401k from a Vault.
 * A badge is minted by the pre_sell method, it must be burned by the sell method.
 */
#[derive(ScryptoSbor, NonFungibleData)]
struct WithdrawBadge {
}

// Internal representation of a user; an item of the buyers KVS.
#[derive(ScryptoSbor, Clone)]
struct User {
    // How many 401k this user owns
    current_bought_amount: Decimal,
    // Weighted average dividends per 401k at buy time for this user
    dividends_per_401k: Decimal,
    // Dividends accrued to this user because of the 401k he sold
    accrued_dividends: Decimal,
    // Next jackpot to accrue to this user
    current_jackpot_number: u32,
    // Past jackpots amount accrued to this user
    accrued_jackpot: Decimal,
}

// Information about a past jackpot; an item of the jackpots KVS
#[derive(ScryptoSbor)]
struct Jackpot {
    // XRD prize per 401k coin
    prize_per_401k: Decimal,
}

// This event is emitted when a user buys some 401k.
#[derive(ScryptoSbor, ScryptoEvent)]
struct BuyEvent {
    // The account the 401k have been deposited in
    account: Global<Account>,
    // Price the 401k were bought at
    price: Decimal,
    // Amount of 401k bought
    bought_amount: Decimal,
    // Current amount of the next jackpot
    current_jackpot_amount: Decimal,
    // Dividends accrued per 401k so far
    global_dividends_per_401k: Decimal,
    // 401k ATH since the last jackpot distribution
    ath: Decimal,
    // Amount of dividends accued to the buyer
    buyer_total_accrued_dividends: Decimal,
    // Past jackpots amount accrued to the buyer
    buyer_accrued_jackpot: Decimal,
}

// This event is emitted when a user sells some 401k.
#[derive(ScryptoSbor, ScryptoEvent)]
struct SellEvent {
    // The account the 401k have been withdrawn from
    account: Global<Account>,
    // Price the 401k were sold at
    price: Decimal,
    // Amount of 401k sold
    sold_amount: Decimal,
    // Current amount of the next jackpot
    current_jackpot_amount: Decimal,
    // Dividends accrued per 401k so far
    global_dividends_per_401k: Decimal,
    // Amount of dividends accued to the seller
    seller_total_accrued_dividends: Decimal,
    // Past jackpots amount accrued to the seller
    seller_accrued_jackpot: Decimal,
}

// This event is emitted when a user withdraws his dividends and jackpot share
#[derive(ScryptoSbor, ScryptoEvent)]
struct WithdrawDividendsEvent {
    // The account whho withdrew dividends
    account: Global<Account>,
    // Amount of withdrawn dividends
    withdrawn_dividends: Decimal,
    // Amount of withdrawn jackpot
    withdrawn_jackpot: Decimal,
}

// This event is emitted when a jackpot is distributed and a new jackpot cycle begins
#[derive(ScryptoSbor, ScryptoEvent)]
struct JackpotDistributedEvent {
    // Amount to distribute
    jackpot_amount: Decimal,
    // Amount per 401k to distribute
    prize_per_401k: Decimal,
}

#[derive(ScryptoSbor, ScryptoEvent)]
struct AirdropCompletedEvent {
    // Dividends accrued per 401k so far
    global_dividends_per_401k: Decimal,
}

#[blueprint]
#[types(
    DepositBadge,
    WithdrawBadge,
    Global<Account>,
    User,
    u32,
    Jackpot,
)]
#[events(
    BuyEvent,
    SellEvent,
    WithdrawDividendsEvent,
    JackpotDistributedEvent,
    AirdropCompletedEvent,
)]
mod club401k {
    struct Club401k {
        // Maximum 401k supply
        max_supply: Decimal,
        // Amount of XRD that are initialy added to the pool when calculating price
        fake_initial_xrd: PreciseDecimal,
        // 401k supply at which the bonding curve changes
        curve_change_supply: Decimal,
        // How much can fake_xrd increase after the bonding curge changes
        price_amplifier: Decimal,
        // XRD pool
        pool: Vault,
        // Vault containing all accrued and not accrued dividends
        dividends: Vault,
        // Percentage of XRD to pay as dividends when buying and selling 401k (0-1 range)
        dividends_percentage: Decimal,
        // Vault containing all past and future jackpot not claimed yet
        jackpot: Vault,
        // Percentage of XRD to pay to the jackpot when buying and selling 401k (0-1 range)
        jackpot_percentage: Decimal,
        // ResourceManager for the 401k deposit badge
        deposit_badge_manager: NonFungibleResourceManager,
        // ResourceManager for the 401k withdraw badge
        withdraw_badge_manager: NonFungibleResourceManager,
        // 401k ResourceManager
        coin_manager: FungibleResourceManager,
        // Next non funglible local id for deposit or withdraw badges
        next_badge_id: u64,
        // Collection of users
        users: KeyValueStore<Global<Account>, User>,
        // Last transaction a deposit or withdraw badge was issued. Use this to avoid issuing both
        // deposit and withdraw badges in the same transaction
        transaction_hash: Hash,
        // Current dividends amount per 401k coin
        dividends_per_401k: Decimal,
        // 401k ATH since the last jackpot distribution
        ath: Decimal,
        // A jackpot distribution can happen when the price is below this percentage of the ATH
        jackpot_threshold: Decimal,
        // How long must the price stay below the threshold for the jackpot to be distributed
        jackpot_threshold_time: i64,
        // Since when the price is below the threshold (i64::MAX if it's currently ower the threshold)
        below_jackpot_threshold_since: i64,
        // Sequence number of the next jackpot distribution
        current_jackpot_number: u32,
        // Current amount of the next jackpot
        current_jackpot_amount: Decimal,
        // Collection of past jackpots
        jackpots: KeyValueStore<u32, Jackpot>,
    }

    impl Club401k {

        /* This function instantiates a globalized Club401k component and creates the resources it
         * will manage
         */
        pub fn new(
            // Percentage of XRD to pay as dividends when buying and selling 401k (0-1 range)
            dividends_percentage: Decimal,
            // Percentage of XRD to pay to the jackpot when buying and selling 401k (0-1 range)
            jackpot_percentage: Decimal,
            // 401k coin initial price
            initial_price: Decimal,
            // 401k coin max supply
            max_supply: Decimal,
            // 401k supply percentage at which the bonding curve changes (0-1 range)
            curve_change_supply_percentage: Decimal,
            // How much can fake_xrd increase after the bonding curge changes
            price_amplifier: Decimal,
            // A jackpot distribution can happen when the price is below this percentage of the ATH
            // (0-1 range)
            jackpot_threshold: Decimal,
            // How long must the price stay below the threshold for the jackpot to be distributed
            jackpot_threshold_time: i64,
        ) -> (
            // Globalized 401k component
            Global<Club401k>,
            // Deposit badge resource address
            ResourceAddress,
            // Wikthdraw badge resource address
            ResourceAddress,
            // 401k coin resource address
            ResourceAddress,
        ) {
            // Check that input parameters make sense
            assert!(
                dividends_percentage >= Decimal::ZERO,
                "Wrong dividends_percentage"
            );
            assert!(
                jackpot_percentage >= Decimal::ZERO,
                "Wrong jackpot_percentage"
            );
            assert!(
                dividends_percentage + jackpot_percentage < Decimal::ONE,
                "dividends_percentage + jackpot_percentage >= 100%"
            );
            assert!(
                initial_price >= Decimal::ZERO,
                "Wrong initial_price"
            );
            assert!(
                max_supply > Decimal::ZERO,
                "Wrong max_supply"
            );
            assert!(
                curve_change_supply_percentage > Decimal::ZERO && curve_change_supply_percentage < Decimal::ONE,
                "Wrong curve_change_supply_percentage"
            );
            assert!(
                price_amplifier >= Decimal::ZERO,
                "Wrong price_amplifier"
            );
            assert!(
                jackpot_threshold > Decimal::ZERO && jackpot_threshold < Decimal::ONE,
                "Wrong jackpot_threshold"
            );

            // Reserve a componet address; it will be used to set roles in the created resources
            let (address_reservation, component_address) =
                Runtime::allocate_component_address(Club401k::blueprint_id());

            // Create the deposit badge resource
            let deposit_badge_manager = ResourceBuilder::new_integer_non_fungible_with_registered_type::<DepositBadge>(
                OwnerRole::None
            )
            .metadata(metadata!(
                roles {
                    metadata_setter => rule!(deny_all);
                    metadata_setter_updater => rule!(deny_all);
                    metadata_locker => rule!(deny_all);
                    metadata_locker_updater => rule!(deny_all);
                },
                init {
                    "name" => "deposit badge", locked;
                }
            ))
            .mint_roles(mint_roles!(
                minter => rule!(require(global_caller(component_address)));
                minter_updater => rule!(deny_all);
            ))
            .burn_roles(burn_roles!(
                burner => rule!(require(global_caller(component_address)));
                burner_updater => rule!(deny_all);
            ))
            .deposit_roles(deposit_roles!(
                depositor => rule!(deny_all);
                depositor_updater => rule!(deny_all);
            ))
            .create_with_no_initial_supply();

            // Get the resource address of the deposit badge
            let deposit_badge_address = deposit_badge_manager.address();

            // Create the withdraw badge resource
            let withdraw_badge_manager = ResourceBuilder::new_integer_non_fungible_with_registered_type::<WithdrawBadge>(
                OwnerRole::None
            )
            .metadata(metadata!(
                roles {
                    metadata_setter => rule!(deny_all);
                    metadata_setter_updater => rule!(deny_all);
                    metadata_locker => rule!(deny_all);
                    metadata_locker_updater => rule!(deny_all);
                },
                init {
                    "name" => "withdraw badge", locked;
                }
            ))
            .mint_roles(mint_roles!(
                minter => rule!(require(global_caller(component_address)));
                minter_updater => rule!(deny_all);
            ))
            .burn_roles(burn_roles!(
                burner => rule!(require(global_caller(component_address)));
                burner_updater => rule!(deny_all);
            ))
            .deposit_roles(deposit_roles!(
                depositor => rule!(deny_all);
                depositor_updater => rule!(deny_all);
            ))
            .create_with_no_initial_supply();

            // Get the resource address of the withdraw badge
            let withdraw_badge_address = withdraw_badge_manager.address();

            // Create the 401k coin resource
            let coin_manager = ResourceBuilder::new_fungible(OwnerRole::None)
            .metadata(metadata!(
                roles {
                    metadata_setter => rule!(deny_all);
                    metadata_setter_updater => rule!(deny_all);
                    metadata_locker => rule!(deny_all);
                    metadata_locker_updater => rule!(deny_all);
                },
                init {
                    "symbol" => "401K", locked;
                    "name" => "401k", locked;
                }
            ))
            .mint_roles(mint_roles!(
                minter => rule!(require(global_caller(component_address)));
                minter_updater => rule!(deny_all);
            ))
            .burn_roles(burn_roles!(
                burner => rule!(require(global_caller(component_address)));
                burner_updater => rule!(deny_all);
            ))
            .withdraw_roles(withdraw_roles!(
                withdrawer => rule!(require(withdraw_badge_address));
                withdrawer_updater => rule!(deny_all);
            ))
            .deposit_roles(deposit_roles!(
                depositor => rule!(require(deposit_badge_address));
                depositor_updater => rule!(deny_all);
            ))
            .create_with_no_initial_supply();

            // Instantiate the component and globalize it
            let component = Self {
                max_supply: max_supply,
                fake_initial_xrd: (initial_price * max_supply).into(),
                curve_change_supply: curve_change_supply_percentage * max_supply,
                price_amplifier: price_amplifier,
                pool: Vault::new(XRD),
                dividends: Vault::new(XRD),
                dividends_percentage: dividends_percentage,
                jackpot: Vault::new(XRD),
                jackpot_percentage: jackpot_percentage,
                deposit_badge_manager: deposit_badge_manager,
                withdraw_badge_manager: withdraw_badge_manager,
                coin_manager: coin_manager,
                next_badge_id: 1,
                users: KeyValueStore::new_with_registered_type(),
                transaction_hash: Runtime::transaction_hash(),
                dividends_per_401k: Decimal::ZERO,
                ath: Decimal::ZERO,
                jackpot_threshold: jackpot_threshold,
                jackpot_threshold_time: jackpot_threshold_time,
                below_jackpot_threshold_since: i64::MAX,
                current_jackpot_number: 1,
                current_jackpot_amount: Decimal::ZERO,
                jackpots: KeyValueStore::new_with_registered_type(),
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::None)
            .with_address(address_reservation)
            .globalize();

            (
                component,
                deposit_badge_address,
                withdraw_badge_address,
                coin_manager.address(),
            )
        }

        /* This method checks that it's never called more than once in the same transaction; it's
         * used to ensure that no more than one badge (withdraw or deposit) is issued in the same
         * transaction.
         */
        fn check_transaction(&mut self) {
            let transaction_hash = Runtime::transaction_hash();

            if transaction_hash == self.transaction_hash {
                Runtime::panic("Multiple operations per transaction".to_string());
            }

            self.transaction_hash = transaction_hash;
        }

        /* Internal method that computes constant product formula with the addition of some fake
         * XRD to set an initial price and to pump when more than curve_change_supply 401k has
         * been minted
         */
        fn constant_product(&self) -> (
            PreciseDecimal,
            PreciseDecimal,
            Decimal,
        ) {
            // Current 401k coin supply
            let current_supply = self.coin_manager.total_supply().unwrap();

            // Compute the number of fake XRD to add to the ones in the pool
            let fake_xrd = match current_supply > self.curve_change_supply {
                // Constant amount up to curve_change_supply
                false => self.fake_initial_xrd,
                // Growing amount after curve_change_supply
                true => self.fake_initial_xrd *
                    (PreciseDecimal::ONE + self.price_amplifier *
                    (current_supply - self.curve_change_supply) /
                    (self.max_supply - self.curve_change_supply)),
            };

            // Total XRD amount to use in the constant product formula
            let xrd_amount = self.pool.amount() + fake_xrd;

            // The constant product
            let constant_product = xrd_amount * (self.max_supply - current_supply);

            (
                constant_product,
                xrd_amount,
                current_supply,
            )
        }

        /* This internal method check if enough time has passed below the threshold to distribute
         * the current jackpot; if so it assigns the current jackpot and starts a new jackpot cycle.
         */
        fn check_jackpot_trigger(
            &mut self,
            // Current 401k coin price
            price: Decimal,
        ) {
            // Proceed only if the price is below the threshold, set the period start to a future
            // date otherwise
            if price > self.jackpot_threshold * self.ath {
                self.below_jackpot_threshold_since = i64::MAX;
                return;
            }

            // Get current time
            let now = Clock::current_time_rounded_to_seconds().seconds_since_unix_epoch;

            // Check that enough time has passed below the threshold
            if now > self.below_jackpot_threshold_since + self.jackpot_threshold_time {

                // Compute the amount of jackpot to associate to each 401k coin
                let prize_per_401k = self.current_jackpot_amount / self.coin_manager.total_supply().unwrap();

                // Emit the JackpotDistributedEvent event
                Runtime::emit_event(
                    JackpotDistributedEvent {
                        jackpot_amount: self.current_jackpot_amount,
                        prize_per_401k: prize_per_401k,
                    }
                );

                // Insert information about the closed jackpot cycle in the KVS
                self.jackpots.insert(
                    self.current_jackpot_number,
                    Jackpot {
                        prize_per_401k: prize_per_401k,
                    }
                );

                // Start a new jackpot cycle
                self.current_jackpot_number += 1;
                self.current_jackpot_amount = Decimal::ZERO;
                self.below_jackpot_threshold_since = i64::MAX;
                self.ath = price;

            // If the period below the the threshold starts now, take note of the current time
            } else if self.below_jackpot_threshold_since == i64::MAX {
                self.below_jackpot_threshold_since = now;
            }
        }

        /* This internal method verifies if one or more jackpots have been distributed since the
         * last operation from buyer; it also returns a bucket with the jackpot share of the buyer
         * if requested.
         */
        fn check_won_jackpots(
            &mut self,
            // The buyer whose pending jackpots are to be found
            user: &mut User,
            // Whether return the jackpot share for the user or not
            withdraw: bool,
        ) -> Option<Bucket> {

            // Initialize a variable to store the jackpot amount for the user
            let mut accrued_jackpot = Decimal::ZERO;

            // For each jackpot distributed after the last operation of this user
            for jackpot_number in user.current_jackpot_number..self.current_jackpot_number {

                // Add his share to the accrued_jackpot variable
                accrued_jackpot += user.current_bought_amount *
                    self.jackpots.get(&jackpot_number).unwrap().prize_per_401k;
            }

            // Has been some pending jackpot share found?
            match accrued_jackpot > Decimal::ZERO {
                // If not, ruturn None
                false => None,
                // If yes, check if the withdraw of the jackpot share has been requested
                true => match withdraw {
                    // If not, accrue the jackpot to the user without returning it, also take note
                    // that the jackpot(s) has been computed for this user
                    false => {
                        user.accrued_jackpot += accrued_jackpot;
                        user.current_jackpot_number = self.current_jackpot_number;
                        None
                    },
                    // If yes, return a bucket with all of the new and old accrued jackpots and
                    // take note that there are no more pending snapshots for this user
                    true => {
                        accrued_jackpot += user.accrued_jackpot;
                        user.accrued_jackpot = Decimal::ZERO;
                        user.current_jackpot_number = self.current_jackpot_number;
                        Some(self.jackpot.take(accrued_jackpot))
                    },
                },
            }       
        }

        fn accrew_dividends(
            &self,
            // The user whose dividends must me accrued
            user: &mut User,
        )  {
            // Accrew the accumulated dividends since the token bought
            user.accrued_dividends += user.current_bought_amount *
                (self.dividends_per_401k - user.dividends_per_401k);

            // No more dividends to accrew
            user.dividends_per_401k = self.dividends_per_401k;
        }

        /* This method exchanges a bucket of XRD for a bucket of 401k coins.
         * A deposit badge is provided so that the user can deposit the 401k coins in his account.
         * The deposit badge must be returned to the post_buy method; it contains information
         * that the post_buy method needs.
         */
        pub fn buy(
            &mut self,
            // XRDs to buy 401k coins
            mut xrd_bucket: Bucket,
        ) -> (
            // 401k coins
            FungibleBucket,
            // deposit badge
            NonFungibleBucket
        ) {
            // Check that no other invocation to buy or pre_sell methods happended in this
            // transaction
            self.check_transaction();

            // Check that the XRD bucket is not empty
            let xrd_amount = xrd_bucket.amount();
            assert!(
                xrd_amount > Decimal::ZERO,
                "No XRD provided"
            );

            // Take the XRD share to use as dividends
            let dividends_amount = xrd_amount * self.dividends_percentage;
            self.dividends.put(
                xrd_bucket.take(dividends_amount)
            );

            // Take the XRD share to add to the jackpot
            let jackpot_amount = xrd_amount * self.jackpot_percentage;
            self.jackpot.put(
                xrd_bucket.take(jackpot_amount)
            );
            self.current_jackpot_amount += jackpot_amount;

            // Get informations needed to compute bought amount
            let (constant_product, mut xrd_in_pool, current_supply) = self.constant_product();

            // Deposit the remainig XRDs in the pool
            let deposited_xrd = xrd_bucket.amount();
            self.pool.put(xrd_bucket);
            xrd_in_pool += deposited_xrd;

            // Compute the bought 401k amount
            let bought_amount = self.max_supply - Decimal::try_from(constant_product / xrd_in_pool).unwrap() - current_supply;

            // Compute the bought price and update ATH information if needed
            let price = deposited_xrd / bought_amount;
            if price > self.ath {
                self.ath = price;
                self.below_jackpot_threshold_since = i64::MAX;
            } else if price > self.ath * self.jackpot_threshold {
                if self.below_jackpot_threshold_since < i64::MAX {
                    self.below_jackpot_threshold_since = i64::MAX;
                }
            } else {
                // Trigger the jackpot distribution if it's the case
                self.check_jackpot_trigger(price);
            }

            // Mint the bought 401k coins
            let coin_bucket = self.coin_manager.mint(bought_amount);

            // Mint the deposit badge
            let deposit_badge_bucket = self.deposit_badge_manager.mint_non_fungible(
                &NonFungibleLocalId::integer(self.next_badge_id.into()),
                DepositBadge {
                    bought_amount: bought_amount,
                    price: price,
                    dividends_amount: dividends_amount,
                }
            );

            (
                coin_bucket,
                deposit_badge_bucket,
            )
        }

        /* This method registers in which account the bought 401k has been deposited and burns the
         * used deposit badge
         */
        pub fn post_buy(
            &mut self,
            // Account where the 401k has been saved
            account: Global<Account>,
            // The deposit badge
            deposit_badge_bucket: NonFungibleBucket,
        ) {
            // Check that the account owner has actually been involved in this transaction
            Runtime::assert_access_rule(account.get_owner_role().rule);

            // Check that the deposit_badge_bucket really contains a deposit badge
            assert!(
                deposit_badge_bucket.resource_address() == self.deposit_badge_manager.address(),
                "Wrong badge"
            );

            // Make sue that exacly one deposit badge is in the deposit_badge_bucket and get its
            // NonFungibleData
            let deposit_badge = deposit_badge_bucket.non_fungible::<DepositBadge>().data();

            // Is the buyer already registered?
            let mut buyer = match self.users.get(&account) {
                // If not create a new one
                None => User {
                    current_bought_amount: Decimal::ZERO,
                    dividends_per_401k: self.dividends_per_401k,
                    accrued_dividends: Decimal::ZERO,
                    current_jackpot_number: self.current_jackpot_number,
                    accrued_jackpot: Decimal::ZERO,
                },
                Some(buyer) => buyer.clone(),
            };

            // Accrew past dividends and jackpots to the buyer
            self.accrew_dividends(&mut buyer);
            _ = self.check_won_jackpots(&mut buyer, false);
            
            // Compute the new global dividends amount per 401k coin
            self.dividends_per_401k +=
                deposit_badge.dividends_amount / self.coin_manager.total_supply().unwrap();

            // Update the bought amount for this buyer
            buyer.current_bought_amount += deposit_badge.bought_amount;

            // Accrew his own share of the dividends he paid to the buyer
            self.accrew_dividends(&mut buyer);

            // Check that the bought 401k have really been deposited in the specified account
            let coin_address = self.coin_manager.address();
            assert!(
                account.balance(coin_address) == buyer.current_bought_amount,
                "Where are the 401ks gone?"
            );

            // Emit the BuyEvent event
            Runtime::emit_event(
                BuyEvent {
                    account: account,
                    price: deposit_badge.price,
                    bought_amount: deposit_badge.bought_amount,
                    current_jackpot_amount: self.current_jackpot_amount,
                    global_dividends_per_401k: self.dividends_per_401k,
                    ath: self.ath,
                    buyer_total_accrued_dividends: buyer.accrued_dividends,
                    buyer_accrued_jackpot: buyer.accrued_jackpot,
                }
            );

            // Update saved buyer information
            self.users.insert(
                account,
                buyer,
            );

            // Burn the deposit badge and be ready to mint the next one
            deposit_badge_bucket.burn();
            self.next_badge_id += 1;
        }

        /* This method mints a withdraw badge that can be used to take some 401k out of and
         * account to sell them.
         * The withdraw badge must be returned to the sell method.
         */
        pub fn pre_sell(&mut self) -> NonFungibleBucket {
            // Check that no other invocation to buy or pre_sell methods happended in this
            // transaction
            self.check_transaction();

            // Mint a withdraw badge
            self.withdraw_badge_manager.mint_non_fungible(
                &NonFungibleLocalId::integer(self.next_badge_id.into()),
                WithdrawBadge {
                }
            )
        }

        /* This method accepts a bucket of 401k coin and swaps them for XRD.
         * It also burns the provided withdraw badge.
         */
        pub fn sell(
            &mut self,
            // Bucket of 401k coins
            coin_bucket: FungibleBucket,
            // The account address of the seller
            account: Global<Account>,
            // The used withdraw badge
            withdraw_badge_bucket: NonFungibleBucket,
        ) -> Bucket {
            // Check that the account owner has actually been involved in this transaction
            Runtime::assert_access_rule(account.get_owner_role().rule);

            // Check that coin_bucket contains a non zero amount of 401k coins
            let coin_address = self.coin_manager.address();
            assert!(
                coin_bucket.resource_address() == coin_address,
                "Wrong coin"
            );
            let coin_amount = coin_bucket.amount();
            assert!(
                coin_amount > Decimal::ZERO,
                "No 401k provided"
            );

            // Check that withdraw_badge_bucket contains exacty one withdraw badge
            assert!(
                withdraw_badge_bucket.resource_address() == self.withdraw_badge_manager.address(),
                "Wrong badge"
            );
            assert!(
                withdraw_badge_bucket.amount() == Decimal::ONE,
                "Exactly one withdraw badge required"
            );

            // Get informations needed to compute the XRD proceeds
            let (constant_product, xrd_in_pool, _) = self.constant_product();

            // Get existing information about the seller
            let mut seller = self.users.get(&account).unwrap().clone();
            
            // Accrew him any past dividends and jackpot
            self.accrew_dividends(&mut seller);
            _ = self.check_won_jackpots(&mut seller, false);
           
            // Check that the 401k coins really came from the specified account
            let withdrawn_401k = seller.current_bought_amount - account.balance(coin_address);
            assert!(
                coin_amount == withdrawn_401k,
                "Where these 401k came from?"
            );

            // Update the account owned coins amount
            seller.current_bought_amount -= withdrawn_401k;
           
            // Burn the sold 401k and get the new supply
            coin_bucket.burn();
            let current_supply = self.coin_manager.total_supply().unwrap();
            
            // Burn the withdraw badge and be ready to mint the next one
            withdraw_badge_bucket.burn();
            self.next_badge_id += 1;

            // Compute the XRD amount from the sale
            let coins_in_pool = self.max_supply - current_supply;
            let new_xrd_in_pool = Decimal::try_from(constant_product / coins_in_pool).unwrap();
            let xrd_amount = Decimal::try_from(xrd_in_pool).unwrap() - new_xrd_in_pool;
            let mut xrd_bucket = self.pool.take(xrd_amount);

            // Take the due dividends out of the XRD and deposit them
            let dividends_amount = xrd_amount * self.dividends_percentage;
            self.dividends.put(
                xrd_bucket.take(dividends_amount)
            );

            // Take the jackpot percentage out of the XRDs
            let jackpot_amount = xrd_amount * self.jackpot_percentage;
            self.jackpot.put(
                xrd_bucket.take(jackpot_amount)
            );
            self.current_jackpot_amount += jackpot_amount;

            // Compute the updated dividends per 401k coin
            if current_supply > Decimal::ZERO {
                self.dividends_per_401k += dividends_amount / current_supply;
            }

            // Compute the sale price and check if a jackpot has been triggered
            let price = xrd_amount / coin_amount;
            if price < self.ath * self.jackpot_threshold {
                self.check_jackpot_trigger(price);
            }

            // Accrew his own share of the dividends he paid to the seller if he still owns some
            // 401k
            if seller.current_bought_amount > Decimal::ZERO {
                self.accrew_dividends(&mut seller);
            }

            // Emit the SellEvent event
            Runtime::emit_event(
                SellEvent {
                    account: account,
                    price: price,
                    sold_amount: coin_amount,
                    current_jackpot_amount: self.current_jackpot_amount,
                    global_dividends_per_401k: self.dividends_per_401k,
                    seller_total_accrued_dividends: seller.accrued_dividends,
                    seller_accrued_jackpot: seller.accrued_jackpot,
                }
            );

            // Save the updated user information
            self.users.insert(
                account,
                seller,
            );

            xrd_bucket
        }

        // This method lets a user receive his pending dividends and jackpot shares
        pub fn withdraw_dividends(
            &mut self,
            // The account to receive dividends and jackpot shares for
            account: Global<Account>,
        ) -> (
            // Dividends
            Bucket,
            // Jackpot shares
            Option<Bucket>,
        ) {
            // Check that the account owner has actually been involved in this transaction
            Runtime::assert_access_rule(account.get_owner_role().rule);

            // Get information about this account
            let mut user = self.users.get(&account).expect("Account not found").clone();

            // Accrew pending dividends to the user
            self.accrew_dividends(&mut user);

            // Take the pending dividends
            let dividends_bucket = self.dividends.take(user.accrued_dividends);

            // Get any new or previously accrued jackpot shares
            let jackpot_bucket = self.check_won_jackpots(&mut user, true);

            // Emit the WithdrawDividendsEvent event
            Runtime::emit_event(
                WithdrawDividendsEvent {
                    account: account,
                    withdrawn_dividends: user.accrued_dividends,
                    withdrawn_jackpot: match jackpot_bucket {
                        None => Decimal::ZERO,
                        Some(ref bucket) => bucket.amount(),
                    }
                }
            );

            // No more pending dividends for this account
            user.accrued_dividends = Decimal::ZERO;

            // Save updated accout information
            self.users.insert(
                account,
                user,
            );

            (
                dividends_bucket,
                jackpot_bucket,
            )
        }

        pub fn airdrop(
            &mut self,
            // XRDs to buy 401k coins
            mut xrd_bucket: Bucket,
            // List of recipients. The sum of the Decimals mst be 1 or less
            recipients: IndexMap<Global<Account>, Decimal>,
        ) {
            // Check that the XRD bucket is not empty
            let xrd_amount = xrd_bucket.amount();
            assert!(
                xrd_amount > Decimal::ZERO,
                "No XRD provided"
            );

            // Take the XRD share to use as dividends
            let dividends_amount = xrd_amount * self.dividends_percentage;
            self.dividends.put(
                xrd_bucket.take(dividends_amount)
            );

            // Take the XRD share to add to the jackpot
            let jackpot_amount = xrd_amount * self.jackpot_percentage;
            self.jackpot.put(
                xrd_bucket.take(jackpot_amount)
            );
            self.current_jackpot_amount += jackpot_amount;

            // Get informations needed to compute bought amount
            let (constant_product, mut xrd_in_pool, current_supply) = self.constant_product();

            // Deposit the remainig XRDs in the pool
            let deposited_xrd = xrd_bucket.amount();
            self.pool.put(xrd_bucket);
            xrd_in_pool += deposited_xrd;

            // Compute the bought 401k amount
            let bought_amount = self.max_supply - Decimal::try_from(constant_product / xrd_in_pool).unwrap() - current_supply;

            // Compute the bought price and update ATH information if needed
            let price = deposited_xrd / bought_amount;
            if price > self.ath {
                self.ath = price;
                self.below_jackpot_threshold_since = i64::MAX;
            } else if price > self.ath * self.jackpot_threshold {
                if self.below_jackpot_threshold_since < i64::MAX {
                    self.below_jackpot_threshold_since = i64::MAX;
                }
            } else {
                // Trigger the jackpot distribution if it's the case
                self.check_jackpot_trigger(price);
            }

            // Mint the bought 401k coins
            let mut coin_bucket = self.coin_manager.mint(bought_amount);

            // Mint the deposit badge
            let deposit_badge_bucket = self.deposit_badge_manager.mint_non_fungible(
                &NonFungibleLocalId::integer(self.next_badge_id.into()),
                DepositBadge {
                    bought_amount: bought_amount,
                    price: price,
                    dividends_amount: dividends_amount,
                }
            );

            // For each recipient
            for (account, share) in recipients.iter() {
                // Compute the amount of 401k for this recipient
                let amount = bought_amount * *share;
                if amount == Decimal::ZERO {
                    continue;
                }

                // Is the recipient already registered?
                let mut recipient = match self.users.get(&account) {
                    // If not create a new one
                    None => User {
                        current_bought_amount: Decimal::ZERO,
                        dividends_per_401k: self.dividends_per_401k,
                        accrued_dividends: Decimal::ZERO,
                        current_jackpot_number: self.current_jackpot_number,
                        accrued_jackpot: Decimal::ZERO,
                    },
                    Some(recipient) => recipient.clone(),
                };

                // Accrew eventual past dividends and jackpots to the recipient
                self.accrew_dividends(&mut recipient);
                _ = self.check_won_jackpots(&mut recipient, false);

                // Try sending the 401k coins to the recipient
                let refund = deposit_badge_bucket.authorize_with_all(
                    || {
                        account.clone().try_deposit_or_refund(
                            coin_bucket.take(amount).into(),
                            None,
                        )
                    }
                );

                // If deposit failed, burn the 401k coins
                if refund.is_some() {
                    refund.unwrap().burn();

                } else {
                    // Update the bought amount for this user
                    recipient.current_bought_amount += amount;

                    // Emit the BuyEvent event
                    Runtime::emit_event(
                        BuyEvent {
                            account: *account,
                            price: price,
                            bought_amount: amount,
                            current_jackpot_amount: self.current_jackpot_amount,
                            global_dividends_per_401k: self.dividends_per_401k,
                            ath: self.ath,
                            buyer_accrued_jackpot: recipient.accrued_jackpot,
                            buyer_total_accrued_dividends: recipient.accrued_dividends,
                        }
                    );
                }

                // Update saved recipient information
                self.users.insert(
                    *account,
                    recipient,
                );
            }

            // Burn the deposit badge and get ready for minting a new one
            deposit_badge_bucket.burn();
            self.next_badge_id += 1;

            // Burn the eventual excess 401k coins and compute the additional dividends amount per
            // 401k coin
            coin_bucket.burn();
            self.dividends_per_401k +=
                dividends_amount / self.coin_manager.total_supply().unwrap();

            // Emit the AirdropCompletedEvent event with the new dividends_per_401k value
            Runtime::emit_event(
                AirdropCompletedEvent {
                    global_dividends_per_401k: self.dividends_per_401k,
                }
            );
        }
    }
}
