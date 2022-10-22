pub struct Listing {
    pub owner_id: AccountId,
    pub approval_id: u64,
    pub token_id: TokenId,
    pub price: u128,
    pub donation: u128
}

pub struct Contract {
    owner_id: AccountId,
    min_price: u128,
    royalty: u128,
    nft_collection: AccountId,
    charity_account: AccountId,
    listings: UnorderedMap<TokenId, Listing>
}

pub type PayoutHashMap = HashMap<AccountId, U128>;

pub struct Payout {
    pub payout: PayoutHashMap,
}

pub struct ListAction {
    price: U128,
    donation: U128
}

impl Contract {
    #[init]
    pub fn new(owner_id: AccountId, min_price: U128, royalty: U128, nft_collection: AccountId, charity_account: AccountId){
        Self {
            owner_id: owner_id,
            min_price: min_price.into(),
            royalty: royalty.into(),
            nft_collection: AccountId,
            charity_account: AccountId,
            listings: UnorderedMap::new("listings")
        }
    }

    fn nft_on_approve(
        &mut self,
        token_id: TokenId,
        owner_id: AccountId,
        approval_id: u64,
        msg: String,
    ) {
        let nft_contract_id = env::predecessor_account_id();
        let signer_id = env::signer_account_id();

        assert_eq!(owner_id, signer_id, "owner_id should be signer_id");
        assert_eq!(nft_contract_id, self.nft_collection, "nft_contract_id is not approved");

        let ListAction {
            price,
            donation
        } = near_sdk::serde_json::from_str(&msg).expect("Invalid price");

        assert!(price.into() > self.min_price, "Can't list token for less than minimum price");

        self.listings.insert(&token_id, Listing {
            owner_id: owner_id,
            approval_id: approval_id,
            token_id: token_id,
            price: price.into(),
            donation: price.into()
        });
    }

    fn update_listing(
        &mut self,
        token_id: TokenId,
        price: U128,
        donation: U128
    ) {
        let listing = self.listings.get(&token_id.clone()).expect("Listing does not exist");
        listing.price = price.into();
        listing.donation = donation.into();
        self.listings.insert(&token_id, &listing);
    }

    fn delete_listing(
        &mut self,
        token_id: TokenId
    ) {
        let listing = self.listings.get(&token_id.clone()).expect("Listing does not exist");
        assert_eq!(listing.owner_id, env::signer_account_id(), "only the owner can delete a listing");
        self.listings.remove(&token_id);
    }

    fn buy(
        &mut self,
        token_id: TokenId
    ) -> Promise {
        let listing = self.listings.get(&token_id.clone()).expect("Listing does not exist");

        assert!(
            env::attached_deposit() >= listing.price,
            "Attached deposit is less than price {}",
            listing.price
        );
        self.listings.remove(&token_id);


        ext_contract::nft_transfer_payout(
            buyer_id.clone(),
            token_id,
            Some(listing.approval_id),
            Some(price.into()),
            Some(10u32),
            nft_contract_id,
            1,
            GAS_FOR_NFT_TRANSFER,
        )
        .then(ext_self::resolve_purchase(
            buyer_id,
            listing,
            env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_ROYALTIES,
        ))
    }

    #[private]
    pub fn resolve_purchase(
        &mut self,
        buyer_id: AccountId,
        listing: Listing,
    ) -> U128 {
        let payout_option = promise_result_as_success().and_then(|value| {
            let parsed_payout = near_sdk::serde_json::from_slice::<Payout>(&value);
            parsed_payout
                .ok()
                .and_then(|payout| {
                    let mut remainder = listing.price;
                    for &value in payout.values() {
                        remainder = remainder.checked_sub(value.0)?;
                    }
                    if remainder <= 100 {
                        Some(payout)
                    } else {
                        None
                    }
                })
            
        });
        let payout = if let Some(payout_option) = payout_option {
            payout_option
        } else {
            if !is_promise_success() {
                Promise::new(buyer_id.clone()).transfer(u128::from(listing.price));
            }
            return price;
        };

        let treasury_fee = listing.price * self.royalty / 10_000u128;

        for (receiver_id, amount) in payout {
            if receiver_id == listing.owner_id {
                Promise::new(receiver_id).transfer(amount - treasury_fee - listing.donation);
                if treasury_fee != 0 {
                    Promise::new(self.owner_id.clone()).transfer(treasury_fee);
                }
                if listing.donation != 0 {
                    Promise::new(self.charity_account.clone()).transfer(listing.donation);
                }
            } else {
                Promise::new(receiver_id).transfer(amount);
            }
        }

        return listing.price;
    }


    // Admin
    pub fn set_owner(&mut self, owner_id: AccountId){
        self.owner_id = owner_id;
    }

    pub fn change_min_price(&mut self, price: U128){
        assert_eq!(env::signer_account_id(), self.owner_id, "Only the owner can change this value");
        self.min_price = price;
    }

    pub fn update_charity_account(&mut self, charity_account: AccountId){
        assert_eq!(env::signer_account_id(), self.owner_id, "Only the owner can change this value");
        self.charity_account = charity_account;
    }

    pub fn update_royalty(&mut self, royalty: U128){
        assert_eq!(env::signer_account_id(), self.owner_id, "Only the owner can change this value");
        self.royalty = royalty;
    }
}