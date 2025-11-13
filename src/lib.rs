use scrypto::prelude::*;

/* NonFungibleData of the badge needed to deposit Jimmi in a Vault.
 * A badge is minted by the buy method, it must be burned by the post_sell
 * method.
 */
#[derive(ScryptoSbor, NonFungibleData)]
struct DepositBadge {
    // Amount of Jimmi bought
    bought_amount: Decimal,
    // Price the Jimmi were bought at
    price: Decimal,
}

/* NonFungibleData of the badge needed to withdraw Jimmi from a Vault.
 * A badge is minted by the pre_sell method, it must be burned by the sell method.
 */
#[derive(ScryptoSbor, NonFungibleData)]
struct WithdrawBadge {
}

// Internal representation of a user; an item of the buyers KVS.
#[derive(ScryptoSbor, Clone)]
struct Buyer {
    // How many Jimmi this user owns
    current_bought_amount: Decimal,
    // Weighted average dividends per Jimmi at buy time for this user
    dividends_per_jimmi: Decimal,
    // Dividends accrued to this user because of the Jimmi he sold
    accrued_dividends: Decimal,
    // Next jackpot to accrue to this user
    current_jackpot_number: u32,
    // Past jackpots amount accrued to this user
    accrued_jackpot: Decimal,
}

// Information about a past jackpot; an item of the jackpots KVS
#[derive(ScryptoSbor)]
struct Jackpot {
    prize_per_jimmi: Decimal,
}

// This event is emitted when a user buys some Jimmi.
#[derive(ScryptoSbor, ScryptoEvent)]
struct BuyEvent {
    // The account the Jimmi have been deposited in
    account: Global<Account>,
    // Price the Jimmi were bought at
    price: Decimal,
    // Amount of Jimmi bought
    bought_amount: Decimal,
    // Amount of dividends not yet accrued to any user
    total_dividends_amount: Decimal,
    // Current amount of the next jackpot
    current_jackpot_amount: Decimal,
    // Current amount of unaccrued dividends per Jimmi
    global_dividends_per_jimmi: Decimal,
    // Weighted average dividends per Jimmi at buy time for the buyer
    buyer_dividends_per_jimmi: Decimal,
    // Jimmi ATH since the last jackpot distribution
    ath: Decimal,
    // Past jackpots amount accrued to the buyer
    buyer_accrued_jackpot: Decimal,
}

// This event is emitted when a user sells some Jimmi.
#[derive(ScryptoSbor, ScryptoEvent)]
struct SellEvent {
    // The account the Jimmi have been withdrawn from
    account: Global<Account>,
    // Price the Jimmi were sold at
    price: Decimal,
    // Amount of Jimmi sold
    sold_amount: Decimal,
    // Amount of dividends not yet accrued to any user
    total_dividends_amount: Decimal,
    // Current amount of the next jackpot
    current_jackpot_amount: Decimal,
    // Current amount of unaccrued dividends per Jimmi
    global_dividends_per_jimmi: Decimal,
    // Amount of dividends accued to the seller
    buyer_total_accrued_dividends: Decimal,
    // Past jackpots amount accrued to the seller
    buyer_accrued_jackpot: Decimal,
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
    // Amount per Jimmi to distribute
    prize_per_jimmi: Decimal,
}

#[blueprint]
#[types(
    DepositBadge,
    WithdrawBadge,
    Global<Account>,
    Buyer,
    u32,
    Jackpot,
)]
#[events(
    BuyEvent,
    SellEvent,
    WithdrawDividendsEvent,
    JackpotDistributedEvent,
)]
mod jimmi {
    struct Jimmi {
        // Maximum Jimmi supply
        max_supply: Decimal,
        // Amount of XRD that are initialy added to the pool when calculating price
        fake_initial_xrd: PreciseDecimal,
        // Jimmi supply at which the bonding curve changes
        curve_change_supply: Decimal,
        // How much can fake_xrd increase after the bonding curge changes
        price_amplifier: Decimal,
        // XRD pool
        pool: Vault,
        // Vault containing all accrued and not accrued dividends
        dividends: Vault,
        // Percentage of XRD to pay as dividends when buying and selling Jimmi (0-1 range)
        dividends_percentage: Decimal,
        // Vault containing all past and future jackpot not claimed yet
        jackpot: Vault,
        // Percentage of XRD to pay to the jackpot when buying and selling Jimmi (0-1 range)
        jackpot_percentage: Decimal,
        // ResourceManager for the Jimmi deposit badge
        deposit_badge_manager: NonFungibleResourceManager,
        // ResourceManager for the Jimmi withdraw badge
        withdraw_badge_manager: NonFungibleResourceManager,
        // Jimmi ResourceManager
        jimmi_manager: FungibleResourceManager,
        // Next non funglible local id for deposit or withdraw badges
        next_badge_id: u64,
        // Collection of users
        buyers: KeyValueStore<Global<Account>, Buyer>,
        // Last transaction a deposit or withdraw badge was issued. Use this to avoid issuing both
        // deposit and withdraw badges in the same transaction
        transaction_hash: Hash,
        // Current amount of dividends not yet accrued to a seller
        unassigned_dividends: Decimal,
        // Jimmi ATH since the last jackpot distribution
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

    impl Jimmi {

        /* This function instantiates a globalized Jimmi component and creates the resources it
         * will manage
         */
        pub fn new(
            // Percentage of XRD to pay as dividends when buying and selling Jimmi (0-1 range)
            dividends_percentage: Decimal,
            // Percentage of XRD to pay to the jackpot when buying and selling Jimmi (0-1 range)
            jackpot_percentage: Decimal,
            // Jimmi coin initial price
            initial_price: Decimal,
            // Jimmi coin max supply
            max_supply: Decimal,
            // Jimmi supply percentage at which the bonding curve changes (0-1 range)
            curve_change_supply_percentage: Decimal,
            // How much can fake_xrd increase after the bonding curge changes
            price_amplifier: Decimal,
            // A jackpot distribution can happen when the price is below this percentage of the ATH
            // (0-1 range)
            jackpot_threshold: Decimal,
            // How long must the price stay below the threshold for the jackpot to be distributed
            jackpot_threshold_time: i64,
        ) -> (
            // Globalized Jimmi component
            Global<Jimmi>,
            // Deposit badge resource address
            ResourceAddress,
            // Wikthdraw badge resource address
            ResourceAddress,
            // Jimmi coin resource address
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
                Runtime::allocate_component_address(Jimmi::blueprint_id());

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

            // Create the Jimmi coin resource
            let jimmi_manager = ResourceBuilder::new_fungible(OwnerRole::None)
            .metadata(metadata!(
                roles {
                    metadata_setter => rule!(deny_all);
                    metadata_setter_updater => rule!(deny_all);
                    metadata_locker => rule!(deny_all);
                    metadata_locker_updater => rule!(deny_all);
                },
                init {
                    "symbol" => "JIMMI", locked;
                    "name" => "Jimmi", locked;
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
                jimmi_manager: jimmi_manager,
                next_badge_id: 1,
                buyers: KeyValueStore::new_with_registered_type(),
                transaction_hash: Runtime::transaction_hash(),
                unassigned_dividends: Decimal::ZERO,
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
                jimmi_manager.address(),
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

        fn constant_product(&self) -> (
            PreciseDecimal,
            PreciseDecimal,
            Decimal,
        ) {
            let current_supply = self.jimmi_manager.total_supply().unwrap();

            let fake_xrd = match current_supply > self.curve_change_supply {
                false => self.fake_initial_xrd,
                true => self.fake_initial_xrd *
                    (PreciseDecimal::ONE + self.price_amplifier *
                    (current_supply - self.curve_change_supply) /
                    (self.max_supply - self.curve_change_supply)),
            };

            let xrd_amount = self.pool.amount() + fake_xrd;

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
            // Current Jimmi coin price
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

                // Compute the amount of jackpot to associate to each Jimmi coin
                let prize_per_jimmi = self.current_jackpot_amount / self.jimmi_manager.total_supply().unwrap();

                // Emit the JackpotDistributedEvent event
                Runtime::emit_event(
                    JackpotDistributedEvent {
                        jackpot_amount: self.current_jackpot_amount,
                        prize_per_jimmi: prize_per_jimmi,
                    }
                );

                // Insert information about the closed jackpot cycle in the KVS
                self.jackpots.insert(
                    self.current_jackpot_number,
                    Jackpot {
                        prize_per_jimmi: prize_per_jimmi,
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
            buyer: &mut Buyer,
            // Whether return the jackpot share for the user or not
            withdraw: bool,
        ) -> Option<Bucket> {

            // Initialize a variable to store the jackpot amount for the user
            let mut accrued_jackpot = Decimal::ZERO;

            // For each jackpot distributed after the last operation of this user
            for jackpot_number in buyer.current_jackpot_number..self.current_jackpot_number {

                // Add his share to the accrued_jackpot variable
                accrued_jackpot += buyer.current_bought_amount *
                    self.jackpots.get(&jackpot_number).unwrap().prize_per_jimmi;
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
                        buyer.accrued_jackpot += accrued_jackpot;
                        buyer.current_jackpot_number = self.current_jackpot_number;
                        None
                    },
                    // If yes, return a bucket with all of the new and old accrued jackpots and
                    // take note that there are no more pending snapshots for this user
                    true => {
                        accrued_jackpot += buyer.accrued_jackpot;
                        buyer.accrued_jackpot = Decimal::ZERO;
                        buyer.current_jackpot_number = self.current_jackpot_number;
                        Some(self.jackpot.take(accrued_jackpot))
                    },
                },
            }       
        }

        /* This method exchanges a bucket of XRD for a bucket of Jimmi coins.
         * A deposit badge is provided so that the user can deposit the Jimmi coins in his account.
         * The deposit badge must be returned to the post_buy method; it contains information
         * that the post_buy method needs.
         */
        pub fn buy(
            &mut self,
            // XRDs to buy Jimmi coins
            mut xrd_bucket: Bucket,
        ) -> (
            // Jimmi coins
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
            self.unassigned_dividends += dividends_amount;

            // Take the XRD share to add to the jackpot
            let jackpot_amount = xrd_amount * self.jackpot_percentage;
            self.jackpot.put(
                xrd_bucket.take(jackpot_amount)
            );
            self.current_jackpot_amount += jackpot_amount;

            let (constant_product, mut xrd_in_pool, current_supply) = self.constant_product();

            // Deposit the remainig XRDs in the pool
            let deposited_xrd = xrd_bucket.amount();
            self.pool.put(xrd_bucket);
            xrd_in_pool += deposited_xrd;

            // Compute the bought Jimmi amount
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

            // Mint the bought Jimmi coins
            let jimmi_bucket = self.jimmi_manager.mint(bought_amount);

            // Mint the deposit badge
            let deposit_badge_bucket = self.deposit_badge_manager.mint_non_fungible(
                &NonFungibleLocalId::integer(self.next_badge_id.into()),
                DepositBadge {
                    bought_amount: bought_amount,
                    price: price,
                }
            );

            (
                jimmi_bucket,
                deposit_badge_bucket,
            )
        }

        /* This method registers in which account the bought Jimmi has been deposited and burns the
         * used deposit badge
         */
        pub fn post_buy(
            &mut self,
            // Account where the Jimmi has been saved
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

            // Compute the new global dividends amount per Jimmi coin
            let global_dividends_per_jimmi =
                self.unassigned_dividends / self.jimmi_manager.total_supply().unwrap();

            // Is the buyer already registered?
            let mut buyer = match self.buyers.get(&account) {
                // If not create a new one
                None => Buyer {
                    current_bought_amount: Decimal::ZERO,
                    dividends_per_jimmi: global_dividends_per_jimmi,
                    accrued_dividends: Decimal::ZERO,
                    current_jackpot_number: self.current_jackpot_number,
                    accrued_jackpot: Decimal::ZERO,
                },
                Some(buyer) => buyer.clone(),
            };

            // Accrew eventual past jackpots to the buyer
            _ = self.check_won_jackpots(&mut buyer, false);

            // Update the dividends_per_jimmi for this buyer
            buyer.dividends_per_jimmi = (buyer.current_bought_amount * buyer.dividends_per_jimmi +
                deposit_badge.bought_amount * global_dividends_per_jimmi) /
                (buyer.current_bought_amount + deposit_badge.bought_amount);

            // Update the bought amount for this buyer
            buyer.current_bought_amount += deposit_badge.bought_amount;

            // Check that the bought Jimmi have really been deposited in the specified account
            let jimmi_address = self.jimmi_manager.address();
            assert!(
                account.balance(jimmi_address) == buyer.current_bought_amount,
                "Where are the jimmis gone?"
            );

            // Emit the BuyEvent event
            Runtime::emit_event(
                BuyEvent {
                    account: account,
                    price: deposit_badge.price,
                    bought_amount: deposit_badge.bought_amount,
                    total_dividends_amount: self.unassigned_dividends,
                    current_jackpot_amount: self.current_jackpot_amount,
                    global_dividends_per_jimmi: global_dividends_per_jimmi,
                    buyer_dividends_per_jimmi: buyer.dividends_per_jimmi,
                    ath: self.ath,
                    buyer_accrued_jackpot: buyer.accrued_jackpot,
                }
            );

            // Update saved buyer information
            self.buyers.insert(
                account,
                buyer,
            );

            // Burn the deposit badge and be ready to mint the next one
            deposit_badge_bucket.burn();
            self.next_badge_id += 1;
        }

        /* This method mints a withdraw badge that can be used to take some Jimmi out of and
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

        /* This method accepts a bucket of Jimmi coin and swaps them for XRD.
         * It also burns the provided withdraw badge.
         */
        pub fn sell(
            &mut self,
            // Bucket of Jimmi coins
            jimmi_bucket: FungibleBucket,
            // The account address of the seller
            account: Global<Account>,
            // The used withdraw badge
            withdraw_badge_bucket: NonFungibleBucket,
        ) -> Bucket {
            // Check that the account owner has actually been involved in this transaction
            Runtime::assert_access_rule(account.get_owner_role().rule);

            // Check that jimmi_bucket contains a non zero amount of Jimmi coins
            let jimmi_address = self.jimmi_manager.address();
            assert!(
                jimmi_bucket.resource_address() == jimmi_address,
                "Wrong coin"
            );
            let jimmi_amount = jimmi_bucket.amount();
            assert!(
                jimmi_amount > Decimal::ZERO,
                "No jimmi provided"
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

            let (constant_product, xrd_in_pool, mut current_supply) = self.constant_product();

            // Get existing information about the seller
            let mut buyer = self.buyers.get(&account).unwrap().clone();
            
            // Accrew him any pending past jackpot
            _ = self.check_won_jackpots(&mut buyer, false);
           
            // Check that the Jimmi coins really came from the specified account
            let withdrawn_jimmi = buyer.current_bought_amount - account.balance(jimmi_address);
            assert!(
                jimmi_amount == withdrawn_jimmi,
                "Where these jimmi came from?"
            );

            // Update the account owned coins amount
            buyer.current_bought_amount -= withdrawn_jimmi;
           
            // Compute the dividends per jimmi before the sale
            let mut global_dividends_per_jimmi = self.unassigned_dividends / current_supply;

            // Accrew the accumulated dividends since the token bought
            let accrued_dividends = global_dividends_per_jimmi * withdrawn_jimmi;
            self.unassigned_dividends -= accrued_dividends;
            buyer.accrued_dividends += accrued_dividends;

            // Burn the sold Jimmi and get the new supply
            jimmi_bucket.burn();
            current_supply = self.jimmi_manager.total_supply().unwrap();
            
            // Burn the withdraw badge and be ready to mint the next one
            withdraw_badge_bucket.burn();
            self.next_badge_id += 1;

            // Compute the XRD amount from the sale
            let jimmi_in_pool = self.max_supply - current_supply;
            let new_xrd_in_pool = Decimal::try_from(constant_product / jimmi_in_pool).unwrap();
            let xrd_amount = Decimal::try_from(xrd_in_pool).unwrap() - new_xrd_in_pool;
            let mut xrd_bucket = self.pool.take(xrd_amount);

            // Take the due dividends out of the XRD and deposit them
            let dividends_amount = xrd_amount * self.dividends_percentage;
            self.dividends.put(
                xrd_bucket.take(dividends_amount)
            );
            self.unassigned_dividends += dividends_amount;

            // Take the jackpot percentage out of the XRDs
            let jackpot_amount = xrd_amount * self.jackpot_percentage;
            self.jackpot.put(
                xrd_bucket.take(jackpot_amount)
            );
            self.current_jackpot_amount += jackpot_amount;

            // Compute the updated dividends per Jimmi coin
            global_dividends_per_jimmi = match current_supply > Decimal::ZERO {
                true => self.unassigned_dividends / current_supply,
                false => Decimal::ZERO,
            };

            // Compute the sale price and check if a jackpot has been triggered
            let price = xrd_amount / jimmi_amount;
            if price < self.ath * self.jackpot_threshold {
                self.check_jackpot_trigger(price);
            }

            // Emit the SellEvent event
            Runtime::emit_event(
                SellEvent {
                    account: account,
                    price: price,
                    sold_amount: jimmi_amount,
                    total_dividends_amount: self.unassigned_dividends,
                    current_jackpot_amount: self.current_jackpot_amount,
                    global_dividends_per_jimmi: global_dividends_per_jimmi,
                    buyer_total_accrued_dividends: buyer.accrued_dividends,
                    buyer_accrued_jackpot: buyer.accrued_jackpot,
                }
            );

            // Save the updated user information
            self.buyers.insert(
                account,
                buyer,
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
            let mut buyer = self.buyers.get(&account).expect("Account not found").clone();

            // Take the pending dividedns and register that there are no more pending dividends for
            // this account
            let dividends_bucket = self.dividends.take(buyer.accrued_dividends);
            buyer.accrued_dividends = Decimal::ZERO;

            // Get any new or previously accrued jackpot shares
            let jackpot_bucket = self.check_won_jackpots(&mut buyer, true);

            // Emit the WithdrawDividendsEvent event
            Runtime::emit_event(
                WithdrawDividendsEvent {
                    account: account,
                    withdrawn_dividends: buyer.accrued_dividends,
                    withdrawn_jackpot: match jackpot_bucket {
                        None => Decimal::ZERO,
                        Some(ref bucket) => bucket.amount(),
                    }
                }
            );

            // Save updated accout information
            self.buyers.insert(
                account,
                buyer,
            );

            (
                dividends_bucket,
                jackpot_bucket,
            )
        }

        pub fn airdrop(
            &mut self,
            // XRDs to buy Jimmi coins
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
            self.unassigned_dividends += dividends_amount;

            // Take the XRD share to add to the jackpot
            let jackpot_amount = xrd_amount * self.jackpot_percentage;
            self.jackpot.put(
                xrd_bucket.take(jackpot_amount)
            );
            self.current_jackpot_amount += jackpot_amount;

            let (constant_product, mut xrd_in_pool, current_supply) = self.constant_product();

            // Deposit the remainig XRDs in the pool
            let deposited_xrd = xrd_bucket.amount();
            self.pool.put(xrd_bucket);
            xrd_in_pool += deposited_xrd;

            // Compute the bought Jimmi amount
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

            // Mint the bought Jimmi coins
            let mut jimmi_bucket = self.jimmi_manager.mint(bought_amount);

            // Mint the deposit badge
            let deposit_badge_bucket = self.deposit_badge_manager.mint_non_fungible(
                &NonFungibleLocalId::integer(self.next_badge_id.into()),
                DepositBadge {
                    bought_amount: bought_amount,
                    price: price,
                }
            );

            // Compute the new global dividends amount per Jimmi coin
            let global_dividends_per_jimmi =
                self.unassigned_dividends / self.jimmi_manager.total_supply().unwrap();

            for (account, share) in recipients.iter() {
                // Is the recipient already registered?
                let mut buyer = match self.buyers.get(&account) {
                    // If not create a new one
                    None => Buyer {
                        current_bought_amount: Decimal::ZERO,
                        dividends_per_jimmi: global_dividends_per_jimmi,
                        accrued_dividends: Decimal::ZERO,
                        current_jackpot_number: self.current_jackpot_number,
                        accrued_jackpot: Decimal::ZERO,
                    },
                    Some(buyer) => buyer.clone(),
                };

                // Compute the amount of Jimmi for this recipient
                let amount = bought_amount * *share;
                if amount == Decimal::ZERO {
                    continue;
                }

                // Accrew eventual past jackpots to the buyer
                _ = self.check_won_jackpots(&mut buyer, false);

                // Try sending the Jiimi to the recipient
                let refund = deposit_badge_bucket.authorize_with_all(
                    || {
                        account.clone().try_deposit_or_refund(
                            jimmi_bucket.take(amount).into(),
                            None,
                        )
                    }
                );

                // If deposit failed, burn the Jimmi coins
                if refund.is_some() {
                    refund.unwrap().burn();

                } else {
                    // Update the dividends_per_jimmi for this recipient
                    buyer.dividends_per_jimmi =
                        (buyer.current_bought_amount * buyer.dividends_per_jimmi +
                        amount * global_dividends_per_jimmi) /
                        (buyer.current_bought_amount + amount);

                    // Update the bought amount for this user
                    buyer.current_bought_amount += amount;

                    // Emit the BuyEvent event
                    Runtime::emit_event(
                        BuyEvent {
                            account: *account,
                            price: price,
                            bought_amount: amount,
                            total_dividends_amount: self.unassigned_dividends,
                            current_jackpot_amount: self.current_jackpot_amount,
                            global_dividends_per_jimmi: global_dividends_per_jimmi,
                            buyer_dividends_per_jimmi: buyer.dividends_per_jimmi,
                            ath: self.ath,
                            buyer_accrued_jackpot: buyer.accrued_jackpot,
                        }
                    );
                }

                // Update saved recipient information
                self.buyers.insert(
                    *account,
                    buyer,
                );
            }

            jimmi_bucket.burn();
            deposit_badge_bucket.burn();
            self.next_badge_id += 1;
        }

    }
}

