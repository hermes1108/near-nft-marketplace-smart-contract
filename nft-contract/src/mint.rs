use crate::*;

#[near_bindgen]
impl Contract {
    #[payable]
    pub fn nft_mint(
        &mut self,
        token_id: Option<TokenId>,
        // metadata: TokenMetadata,
        receiver_id: AccountId,
        //we add an optional parameter for perpetual royalties
        perpetual_royalties: Option<HashMap<AccountId, u32>>,
    ) -> String{
        
        let total_supply = self.token_metadata_by_id.len();
        if total_supply >= 2000 {
            return String::from("Supply exceeds");
        }
        let my_token_id = RAND_TOKEN_IDS[total_supply as usize];
        let token_type = get_type_by_id(my_token_id);
        let caller = env::predecessor_account_id();
        let deposit = env::attached_deposit();
        let curr_time = env::block_timestamp() / 1_000_000;

        let metadata = TokenMetadata {
            title: Some(format!("NEAR Labs #{}", my_token_id)), // ex. "Arch Nemesis: Mail Carrier" or "Parcel #5055"
            description: Some(String::from("This is simple description of NEAR Labs.")), // free-form description
            media: Some(format!("http://ipfs.io/ipfs/QmVboBxPPQNATwu9iSTL66KKTG2sBDDyWFZr7EBvDpNk3q/{}.png", token_type)), // URL to associated media, preferably to decentralized, content-addressed storage
            media_hash: Some(Base64VecU8(b"VGhpcyBpcyBtZWRpYSBoYXNoLg==".to_vec())), // Base64-encoded sha256 hash of content referenced by the `media` field. Required if `media` is included.
            copies: Some(1), // number of copies of this set of metadata in existence when token was minted.
            issued_at: Some(curr_time), // When token was issued or minted, Unix epoch in milliseconds
            expires_at: None, // When token expires, Unix epoch in milliseconds
            starts_at: None, // When token starts being valid, Unix epoch in milliseconds
            updated_at: None, // When token was last updated, Unix epoch in milliseconds
            extra: Some(format!("{{\"attributes\": [{{\"trait_type\": \"Class\", \"value\": \"{}\" }}]}}", token_type)), // anything extra the NFT wants to store on-chain. Can be stringified JSON.
            reference: Some(format!("https://ipfs.io/ipfs/QmRiLKmhizpnwqpHGeiJnL4G6fsPAxdEdCiDkuJpt7xHPH/{}.json", my_token_id)), // URL to an off-chain JSON file with more info.
            reference_hash: Some(Base64VecU8(b"QmFzZTY0LWVuY29kZWQgc2hhMjU2IGhhc2ggb2YgSlNPTiBmcm9tIHJlZmVyZW5jZSBmaWVsZC4=".to_vec())), // Base64-encoded sha256 hash of JSON from reference field. Required if `reference` is included.
        };
        
        const PRESALE_TIME: u64 = 1653231348000; // 13th June 2022 04:00PM UTC
        const PUBSALE_TIME: u64 = 1655139600000; // 13th June 2022 05:00PM UTC
        const PRESALE_PRICE: u128 = 1_000_000_000_000_000_000_000_000; // 1 $NEAR
        const PUBSALE_PRICE: u128 = 2_000_000_000_000_000_000_000_000; // 2 $NEAR
        if curr_time < PRESALE_TIME {
            return String::from("Presale not started");
        } else if curr_time > PRESALE_TIME && curr_time < PUBSALE_TIME {
            if !self.whitelist.contains_key(&caller){
                return String::from("Not a whitelist");
            }
            if deposit < PRESALE_PRICE {
                return String::from("insufficient fund for presale");
            }
        }
        else if deposit < PUBSALE_PRICE {
            return String::from("insufficient fund for public sale");
        }

        let mut final_token_id = format!("{}", my_token_id);
        if let Some(token_id) = token_id {
            final_token_id = token_id;
        }
        //measure the initial storage being used on the contract
        let initial_storage_usage = env::storage_usage();

        // create a royalty map to store in the token
        let mut royalty = HashMap::new();

        // if perpetual royalties were passed into the function: 
        if let Some(perpetual_royalties) = perpetual_royalties {
            //make sure that the length of the perpetual royalties is below 7 since we won't have enough GAS to pay out that many people
            assert!(perpetual_royalties.len() < 7, "Cannot add more than 6 perpetual royalty amounts");

            //iterate through the perpetual royalties and insert the account and amount in the royalty map
            for (account, amount) in perpetual_royalties {
                royalty.insert(account, amount);
            }
        }

        //specify the token struct that contains the owner ID 
        let token = Token {
            //set the owner ID equal to the receiver ID passed into the function
            owner_id: receiver_id,
            //we set the approved account IDs to the default value (an empty map)
            approved_account_ids: Default::default(),
            //the next approval ID is set to 0
            next_approval_id: 0,
            //the map of perpetual royalties for the token (The owner will get 100% - total perpetual royalties)
            royalty,
        };

        //insert the token ID and token struct and make sure that the token doesn't exist
        assert!(
            self.tokens_by_id.insert(&final_token_id, &token).is_none(),
            "Token already exists"
        );

        //insert the token ID and metadata
        self.token_metadata_by_id.insert(&final_token_id, &metadata);

        //call the internal method for adding the token to the owner
        self.internal_add_token_to_owner(&token.owner_id, &final_token_id);

        // Construct the mint log as per the events standard.
        let nft_mint_log: EventLog = EventLog {
            // Standard name ("nep171").
            standard: NFT_STANDARD_NAME.to_string(),
            // Version of the standard ("nft-1.0.0").
            version: NFT_METADATA_SPEC.to_string(),
            // The data related with the event stored in a vector.
            event: EventLogVariant::NftMint(vec![NftMintLog {
                // Owner of the token.
                owner_id: token.owner_id.to_string(),
                // Vector of token IDs that were minted.
                token_ids: vec![final_token_id.to_string()],
                // An optional memo to include.
                memo: None,
            }]),
        };

        // Log the serialized json.
        env::log_str(&nft_mint_log.to_string());

        //calculate the required storage which was the used - initial
        let required_storage_in_bytes = env::storage_usage() - initial_storage_usage;

        //refund any excess storage if the user attached too much. Panic if they didn't attach enough to cover the required.
        refund_deposit(required_storage_in_bytes);
        nft_mint_log.to_string()
    }

    pub fn add_whitelist(&mut self, account_id: AccountId) {
        self.assert_owner();
        self.whitelist.insert(&account_id, &(true));
    }

    pub fn remove_whitelist(&mut self, account_id: AccountId) {
        self.assert_owner();
        self.whitelist.remove(&account_id);
    }

    pub fn is_whitelist(&self, account_id: AccountId) -> bool {
        return self.whitelist.contains_key(&account_id);
    }

    pub fn get_curr_time(&self) -> u64 {
        return env::block_timestamp() / 1_000_000;
    }
}