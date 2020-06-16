use crate::lightwallet::LightWallet;

use std::sync::{Arc, RwLock, Mutex};
use std::io;
use std::io::prelude::*;
use std::io::{ErrorKind};

use rand::{Rng, rngs::OsRng};

use json::{object, array, JsonValue};
use zcash_client_backend::{
    constants::testnet, constants::mainnet, constants::regtest, encoding::encode_payment_address,
};

use log::{info, warn, error};

use crate::grpcconnector::{self, *};
use crate::SaplingParams;
use crate::ANCHOR_OFFSET;

mod checkpoints;

pub const DEFAULT_SERVER: &str = "https://lightwalletd.ycash.xyz:443";

#[derive(Clone, Debug)]
pub struct WalletStatus {
    pub is_syncing: bool,
    pub total_blocks: u64,
    pub synced_blocks: u64,
}

impl WalletStatus {
    pub fn new() -> Self {
        WalletStatus {
            is_syncing: false,
            total_blocks: 0,
            synced_blocks: 0
        }
    }
}

#[derive(Clone, Debug)]
pub struct LightClientConfig {
    pub server                      : http::Uri,
    pub chain_name                  : String,
    pub sapling_activation_height   : u64,
    pub consensus_branch_id         : String,
    pub anchor_offset               : u32,
    pub no_cert_verification        : bool,
    pub data_dir                    : Option<String>
}

impl LightClientConfig {

    // Create an unconnected (to any server) config to test for local wallet etc...
    pub fn create_unconnected(chain_name: String, dir: Option<String>) -> LightClientConfig {
        LightClientConfig {
            server                      : http::Uri::default(),
            chain_name                  : chain_name,
            sapling_activation_height   : 0,
            consensus_branch_id         : "".to_string(),
            anchor_offset               : ANCHOR_OFFSET,
            no_cert_verification        : false,
            data_dir                    : dir,
        }
    }

    pub fn create(server: http::Uri, dangerous: bool) -> io::Result<(LightClientConfig, u64)> {
        use std::net::ToSocketAddrs;
        // Test for a connection first
        format!("{}:{}", server.host().unwrap(), server.port_part().unwrap())
            .to_socket_addrs()?
            .next()
            .ok_or(std::io::Error::new(ErrorKind::ConnectionRefused, "Couldn't resolve server!"))?;

        // Do a getinfo first, before opening the wallet
        let info = grpcconnector::get_info(server.clone(), dangerous)
            .map_err(|e| std::io::Error::new(ErrorKind::ConnectionRefused, e))?;

        // Create a Light Client Config
        let config = LightClientConfig {
            server,
            chain_name                  : info.chain_name,
            sapling_activation_height   : info.sapling_activation_height,
            consensus_branch_id         : info.consensus_branch_id,
            anchor_offset               : ANCHOR_OFFSET,
            no_cert_verification        : dangerous,
            data_dir                    : None,
        };

        Ok((config, info.block_height))
    }

    pub fn get_initial_state(&self, height: u64) -> Option<(u64, &str, &str)> {
        checkpoints::get_closest_checkpoint(&self.chain_name, height)
    }

    pub fn get_server_or_default(server: Option<String>) -> http::Uri {
        match server {
            Some(s) => {
                let mut s = if s.starts_with("http") {s} else { "http://".to_string() + &s};
                let uri: http::Uri = s.parse().unwrap();
                if uri.port_part().is_none() {
                    s = s + ":443";
                }
                s
            }
            None    => DEFAULT_SERVER.to_string()
        }.parse().unwrap()
    }

    pub fn get_coin_type(&self) -> u32 {
        match &self.chain_name[..] {
            "main"    => mainnet::COIN_TYPE,
            "test"    => testnet::COIN_TYPE,
            "regtest" => regtest::COIN_TYPE,
            c         => panic!("Unknown chain {}", c)
        }
    }

    pub fn hrp_sapling_address(&self) -> &str {
        match &self.chain_name[..] {
            "main"    => mainnet::HRP_SAPLING_PAYMENT_ADDRESS,
            "test"    => testnet::HRP_SAPLING_PAYMENT_ADDRESS,
            "regtest" => regtest::HRP_SAPLING_PAYMENT_ADDRESS,
            c         => panic!("Unknown chain {}", c)
        }
    }

    pub fn hrp_sapling_private_key(&self) -> &str {
        match &self.chain_name[..] {
            "main"    => mainnet::HRP_SAPLING_EXTENDED_SPENDING_KEY,
            "test"    => testnet::HRP_SAPLING_EXTENDED_SPENDING_KEY,
            "regtest" => regtest::HRP_SAPLING_EXTENDED_SPENDING_KEY,
            c         => panic!("Unknown chain {}", c)
        }
    }

    pub fn base58_pubkey_address(&self) -> [u8; 2] {
        match &self.chain_name[..] {
            "main"    => mainnet::B58_PUBKEY_ADDRESS_PREFIX,
            "test"    => testnet::B58_PUBKEY_ADDRESS_PREFIX,
            "regtest" => regtest::B58_PUBKEY_ADDRESS_PREFIX,
            c         => panic!("Unknown chain {}", c)
        }
    }


    pub fn base58_script_address(&self) -> [u8; 2] {
        match &self.chain_name[..] {
            "main"    => mainnet::B58_SCRIPT_ADDRESS_PREFIX,
            "test"    => testnet::B58_SCRIPT_ADDRESS_PREFIX,
            "regtest" => regtest::B58_SCRIPT_ADDRESS_PREFIX,
            c         => panic!("Unknown chain {}", c)
        }
    }

    pub fn base58_secretkey_prefix(&self) -> [u8; 1] {
        match &self.chain_name[..] {
            "main"    => [0x80],
            "test"    => [0xEF],
            "regtest" => [0xEF],
            c         => panic!("Unknown chain {}", c)
        }
    }
}

pub struct LightClient {
    pub wallet          : Arc<RwLock<LightWallet>>,

    pub config          : LightClientConfig,

    // zcash-params
    pub sapling_output  : Vec<u8>,
    pub sapling_spend   : Vec<u8>,

    sync_lock           : Mutex<()>,
    sync_status         : Arc<RwLock<WalletStatus>>, // The current syncing status of the Wallet.
}

impl LightClient {
    
    pub fn set_wallet_initial_state(&self, height: u64) {
        use std::convert::TryInto;

        let state = self.config.get_initial_state(height);

        match state {
            Some((height, hash, tree)) => self.wallet.read().unwrap().set_initial_block(height.try_into().unwrap(), hash, tree),
            _ => true,
        };
    }

    fn read_sapling_params(&mut self) {
        // Read Sapling Params
        self.sapling_output.extend_from_slice(SaplingParams::get("sapling-output.params").unwrap().as_ref());
        self.sapling_spend.extend_from_slice(SaplingParams::get("sapling-spend.params").unwrap().as_ref());

    }

    /// Method to create a test-only version of the LightClient
    #[allow(dead_code)]
    pub fn unconnected(seed_phrase: String, dir: Option<String>) -> io::Result<Self> {
        let config = LightClientConfig::create_unconnected("test".to_string(), dir);
        let mut l = LightClient {
                wallet          : Arc::new(RwLock::new(LightWallet::new(Some(seed_phrase), &config, 0)?)),
                config          : config.clone(),
                sapling_output  : vec![], 
                sapling_spend   : vec![],
                sync_lock       : Mutex::new(()),
                sync_status     : Arc::new(RwLock::new(WalletStatus::new())),
            };

        l.set_wallet_initial_state(0);
        l.read_sapling_params();

        info!("Created new wallet!");
        info!("Created LightClient to {}", &config.server);

        Ok(l)
    }

    /// Create a brand new wallet with a new seed phrase. 
    pub fn new(config: &LightClientConfig, latest_block: u64) -> io::Result<Self> {
        let mut l = LightClient {
                wallet          : Arc::new(RwLock::new(LightWallet::new(None, config, latest_block)?)),
                config          : config.clone(),
                sapling_output  : vec![], 
                sapling_spend   : vec![],
                sync_lock       : Mutex::new(()),
                sync_status     : Arc::new(RwLock::new(WalletStatus::new())),
            };

        l.set_wallet_initial_state(latest_block);
        l.read_sapling_params();

        info!("Created new wallet with a new seed!");
        info!("Created LightClient to {}", &config.server);

        Ok(l)
    }

    pub fn new_from_phrase(seed_phrase: String, config: &LightClientConfig, birthday: u64) -> io::Result<Self> {
        let mut l = LightClient {
                wallet          : Arc::new(RwLock::new(LightWallet::new(Some(seed_phrase), config, birthday)?)),
                config          : config.clone(),
                sapling_output  : vec![], 
                sapling_spend   : vec![],
                sync_lock       : Mutex::new(()),
                sync_status     : Arc::new(RwLock::new(WalletStatus::new())),
            };

        println!("Setting birthday to {}", birthday);
        l.set_wallet_initial_state(birthday);
        l.read_sapling_params();

        info!("Created new wallet!");
        info!("Created LightClient to {}", &config.server);

        Ok(l)
    }

    pub fn read_from_buffer<R: Read>(config: &LightClientConfig, mut reader: R) -> io::Result<Self>{
        let wallet = LightWallet::read(&mut reader, config)?;
        let mut lc = LightClient {
            wallet          : Arc::new(RwLock::new(wallet)),
            config          : config.clone(),
            sapling_output  : vec![], 
            sapling_spend   : vec![],
            sync_lock       : Mutex::new(()),
            sync_status     : Arc::new(RwLock::new(WalletStatus::new())),
        };

        lc.read_sapling_params();

        info!("Read wallet with birthday {}", lc.wallet.read().unwrap().get_first_tx_block());
        info!("Created LightClient to {}", &config.server);

        Ok(lc)
    }

    pub fn last_scanned_height(&self) -> u64 {
        self.wallet.read().unwrap().last_scanned_height() as u64
    }

    // Export private keys
    pub fn do_export(&self, addr: Option<String>) -> Result<JsonValue, &str> {
        if !self.wallet.read().unwrap().is_unlocked_for_spending() {
            error!("Wallet is locked");
            return Err("Wallet is locked");
        }

        // Clone address so it can be moved into the closure
        let address = addr.clone();
        let wallet = self.wallet.read().unwrap();
        // Go over all z addresses
        let z_keys = wallet.get_z_private_keys().iter()
            .filter( move |(addr, _)| address.is_none() || address.as_ref() == Some(addr))
            .map( |(addr, pk)|
                object!{
                    "address"     => addr.clone(),
                    "private_key" => pk.clone()
                }
            ).collect::<Vec<JsonValue>>();

        // Clone address so it can be moved into the closure
        let address = addr.clone();

        // Go over all t addresses
        let t_keys = wallet.get_t_secret_keys().iter()
            .filter( move |(addr, _)| address.is_none() || address.as_ref() == Some(addr))
            .map( |(addr, sk)|
                object!{
                    "address"     => addr.clone(),
                    "private_key" => sk.clone(),
                }
            ).collect::<Vec<JsonValue>>();

        let mut all_keys = vec![];
        all_keys.extend_from_slice(&z_keys);
        all_keys.extend_from_slice(&t_keys);

        Ok(all_keys.into())
    }

    pub fn do_address(&self) -> JsonValue {
        let wallet = self.wallet.read().unwrap();

        // Collect z addresses
        let z_addresses = wallet.zaddress.read().unwrap().iter().map( |ad| {
            encode_payment_address(self.config.hrp_sapling_address(), &ad)
        }).collect::<Vec<String>>();

        // Collect t addresses
        let t_addresses = wallet.taddresses.read().unwrap().iter().map( |a| a.clone() )
                            .collect::<Vec<String>>();

        object!{
            "z_addresses" => z_addresses,
            "t_addresses" => t_addresses,
        }
    }

    pub fn do_balance(&self) -> JsonValue {
        let wallet = self.wallet.read().unwrap();

        // Collect z addresses
        let z_addresses = wallet.zaddress.read().unwrap().iter().map( |ad| {
            let address = encode_payment_address(self.config.hrp_sapling_address(), &ad);
            object!{
                "address" => address.clone(),
                "zbalance" => wallet.zbalance(Some(address.clone())),
                "verified_zbalance" => wallet.verified_zbalance(Some(address)),
            }
        }).collect::<Vec<JsonValue>>();

        // Collect t addresses
        let t_addresses = wallet.taddresses.read().unwrap().iter().map( |address| {
            // Get the balance for this address
            let balance = wallet.tbalance(Some(address.clone()));
            
            object!{
                "address" => address.clone(),
                "balance" => balance,
            }
        }).collect::<Vec<JsonValue>>();

        object!{
            "zbalance"           => wallet.zbalance(None),
            "verified_zbalance"  => wallet.verified_zbalance(None),
            "tbalance"           => wallet.tbalance(None),
            "z_addresses"        => z_addresses,
            "t_addresses"        => t_addresses,
        }
    }

    pub fn do_save_to_buffer(&self) -> Result<Vec<u8>, String> {
        // If the wallet is encrypted but unlocked, lock it again.
        {
           let mut wallet = self.wallet.write().unwrap();
           if wallet.is_encrypted() && wallet.is_unlocked_for_spending() {
               match wallet.lock() {
                   Ok(_) => {},
                   Err(e) => {
                       let err = format!("ERR: {}", e);
                       error!("{}", err);
                       return Err(e.to_string());
                   }
               }
           }
       }        

       let mut buffer: Vec<u8> = vec![];
       match self.wallet.write().unwrap().write(&mut buffer) {
           Ok(_) => Ok(buffer),
           Err(e) => {
               let err = format!("ERR: {}", e);
               error!("{}", err);
               Err(e.to_string())
           }
       }
   }

    pub fn get_server_uri(&self) -> http::Uri {
        self.config.server.clone()
    }

    pub fn do_info(&self) -> String {
        match get_info(self.get_server_uri(), self.config.no_cert_verification) {
            Ok(i) => {
                let o = object!{
                    "version" => i.version,
                    "vendor" => i.vendor,
                    "taddr_support" => i.taddr_support,
                    "chain_name" => i.chain_name,
                    "sapling_activation_height" => i.sapling_activation_height,
                    "consensus_branch_id" => i.consensus_branch_id,
                    "latest_block_height" => i.block_height
                };
                o.pretty(2)
            },
            Err(e) => e
        }
    }

    pub fn do_seed_phrase(&self) -> Result<JsonValue, &str> {
        if !self.wallet.read().unwrap().is_unlocked_for_spending() {
            error!("Wallet is locked");
            return Err("Wallet is locked");
        }

        let wallet = self.wallet.read().unwrap();
        Ok(object!{
            "seed"     => wallet.get_seed_phrase(),
            "birthday" => wallet.get_birthday()
        })
    }

    // Return a list of all notes, spent and unspent
    pub fn do_list_notes(&self, all_notes: bool) -> JsonValue {
        let mut unspent_notes: Vec<JsonValue> = vec![];
        let mut spent_notes  : Vec<JsonValue> = vec![];
        let mut pending_notes: Vec<JsonValue> = vec![];

        {
            // Collect Sapling notes
            let wallet = self.wallet.read().unwrap();
            wallet.txs.read().unwrap().iter()
                .flat_map( |(txid, wtx)| {
                    wtx.notes.iter().filter_map(move |nd| 
                        if !all_notes && nd.spent.is_some() {
                            None
                        } else {
                            Some(object!{
                                "created_in_block"   => wtx.block,
                                "datetime"           => wtx.datetime,
                                "created_in_txid"    => format!("{}", txid),
                                "value"              => nd.note.value,
                                "is_change"          => nd.is_change,
                                "address"            => LightWallet::note_address(self.config.hrp_sapling_address(), nd),
                                "spent"              => nd.spent.map(|spent_txid| format!("{}", spent_txid)),
                                "unconfirmed_spent"  => nd.unconfirmed_spent.map(|spent_txid| format!("{}", spent_txid)),
                            })
                        }
                    )
                })
                .for_each( |note| {
                    if note["spent"].is_null() && note["unconfirmed_spent"].is_null() {
                        unspent_notes.push(note);
                    } else if !note["spent"].is_null() {
                        spent_notes.push(note);
                    } else {
                        pending_notes.push(note);
                    }
                });
        }
        
        let mut unspent_utxos: Vec<JsonValue> = vec![];
        let mut spent_utxos  : Vec<JsonValue> = vec![];
        let mut pending_utxos: Vec<JsonValue> = vec![];
        
        {
            let wallet = self.wallet.read().unwrap();
            wallet.txs.read().unwrap().iter()
                .flat_map( |(txid, wtx)| {
                    wtx.utxos.iter().filter_map(move |utxo| 
                        if !all_notes && utxo.spent.is_some() {
                            None
                        } else {
                            Some(object!{
                                "created_in_block"   => wtx.block,
                                "datetime"           => wtx.datetime,
                                "created_in_txid"    => format!("{}", txid),
                                "value"              => utxo.value,
                                "scriptkey"          => hex::encode(utxo.script.clone()),
                                "is_change"          => false, // TODO: Identify notes as change if we send change to taddrs
                                "address"            => utxo.address.clone(),
                                "spent"              => utxo.spent.map(|spent_txid| format!("{}", spent_txid)),
                                "unconfirmed_spent"  => utxo.unconfirmed_spent.map(|spent_txid| format!("{}", spent_txid)),
                            })
                        }
                    )
                })
                .for_each( |utxo| {
                    if utxo["spent"].is_null() && utxo["unconfirmed_spent"].is_null() {
                        unspent_utxos.push(utxo);
                    } else if !utxo["spent"].is_null() {
                        spent_utxos.push(utxo);
                    } else {
                        pending_utxos.push(utxo);
                    }
                });
        }

        let mut res = object!{
            "unspent_notes" => unspent_notes,
            "pending_notes" => pending_notes,
            "utxos"         => unspent_utxos,
            "pending_utxos" => pending_utxos,
        };

        if all_notes {
            res["spent_notes"] = JsonValue::Array(spent_notes);
            res["spent_utxos"] = JsonValue::Array(spent_utxos);
        }

        res
    }

    pub fn do_encryption_status(&self) -> JsonValue {
        let wallet = self.wallet.read().unwrap();
        object!{
            "encrypted" => wallet.is_encrypted(),
            "locked"    => !wallet.is_unlocked_for_spending()
        }
    }

    pub fn do_list_transactions(&self) -> JsonValue {
        let wallet = self.wallet.read().unwrap();

        // Create a list of TransactionItems from wallet txns
        let mut tx_list = wallet.txs.read().unwrap().iter()
            .flat_map(| (_k, v) | {
                let mut txns: Vec<JsonValue> = vec![];

                if v.total_shielded_value_spent + v.total_transparent_value_spent > 0 {
                    // If money was spent, create a transaction. For this, we'll subtract
                    // all the change notes. TODO: Add transparent change here to subtract it also
                    let total_change: u64 = v.notes.iter()
                        .filter( |nd| nd.is_change )
                        .map( |nd| nd.note.value )
                        .sum();

                    // TODO: What happens if change is > than sent ?

                    // Collect outgoing metadata
                    let outgoing_json = v.outgoing_metadata.iter()
                        .map(|om| 
                            object!{
                                "address" => om.address.clone(),
                                "value"   => om.value,
                                "memo"    => LightWallet::memo_str(&Some(om.memo.clone())),
                        })
                        .collect::<Vec<JsonValue>>();                    

                    txns.push(object! {
                        "block_height" => v.block,
                        "datetime"     => v.datetime,
                        "txid"         => format!("{}", v.txid),
                        "amount"       => total_change as i64 
                                            - v.total_shielded_value_spent as i64 
                                            - v.total_transparent_value_spent as i64,
                        "outgoing_metadata" => outgoing_json,
                    });
                } 

                // For each sapling note that is not a change, add a Tx.
                txns.extend(v.notes.iter()
                    .filter( |nd| !nd.is_change )
                    .map ( |nd| 
                        object! {
                            "block_height" => v.block,
                            "datetime"     => v.datetime,
                            "txid"         => format!("{}", v.txid),
                            "amount"       => nd.note.value as i64,
                            "address"      => LightWallet::note_address(self.config.hrp_sapling_address(), nd),
                            "memo"         => LightWallet::memo_str(&nd.memo),
                    })
                );

                // Get the total transparent received
                let total_transparent_received = v.utxos.iter().map(|u| u.value).sum::<u64>();
                if total_transparent_received > v.total_transparent_value_spent {
                    // Create an input transaction for the transparent value as well.
                    txns.push(object!{
                        "block_height" => v.block,
                        "datetime"     => v.datetime,
                        "txid"         => format!("{}", v.txid),
                        "amount"       => total_transparent_received as i64 - v.total_transparent_value_spent as i64,
                        "address"      => v.utxos.iter().map(|u| u.address.clone()).collect::<Vec<String>>().join(","),
                        "memo"         => None::<String>
                    })
                }

                txns
            })
            .collect::<Vec<JsonValue>>();

        // Add in all mempool txns
        tx_list.extend(wallet.mempool_txs.read().unwrap().iter().map( |(_, wtx)| {
            use zcash_primitives::transaction::components::amount::DEFAULT_FEE;
            use std::convert::TryInto;
            
            let amount: u64 = wtx.outgoing_metadata.iter().map(|om| om.value).sum::<u64>();
            let fee: u64 = DEFAULT_FEE.try_into().unwrap();

            // Collect outgoing metadata
            let outgoing_json = wtx.outgoing_metadata.iter()
                .map(|om| 
                    object!{
                        "address" => om.address.clone(),
                        "value"   => om.value,
                        "memo"    => LightWallet::memo_str(&Some(om.memo.clone())),
                }).collect::<Vec<JsonValue>>();                    

            object! {
                "block_height" => wtx.block,
                "datetime"     => wtx.datetime,
                "txid"         => format!("{}", wtx.txid),
                "amount"       => -1 * (fee + amount) as i64,
                "unconfirmed"  => true,
                "outgoing_metadata" => outgoing_json,
            }
        }));

        tx_list.sort_by( |a, b| if a["block_height"] == b["block_height"] {
                                    a["txid"].as_str().cmp(&b["txid"].as_str())
                                } else {
                                    a["block_height"].as_i32().cmp(&b["block_height"].as_i32())
                                }
        );

        JsonValue::Array(tx_list)
    }

    /// Create a new address, deriving it from the seed.
    pub fn do_new_address(&self, addr_type: &str) -> Result<JsonValue, String> {
        if !self.wallet.read().unwrap().is_unlocked_for_spending() {
            error!("Wallet is locked");
            return Err("Wallet is locked".to_string());
        }

        let new_address = {
            let wallet = self.wallet.write().unwrap();

            match addr_type {
                "z" => wallet.add_zaddr(),
                "t" => wallet.add_taddr(),
                _   => {
                    let e = format!("Unrecognized address type: {}", addr_type);
                    error!("{}", e);
                    return Err(e);
                }
            }
        };

        Ok(array![new_address])
    }

    pub fn clear_state(&self) {
        // First, clear the state from the wallet
        self.wallet.read().unwrap().clear_blocks();

        // Then set the initial block
        self.set_wallet_initial_state(self.wallet.read().unwrap().get_birthday());
        info!("Cleared wallet state");        
    }

    pub fn do_rescan(&self) -> Result<JsonValue, String> {
        if !self.wallet.read().unwrap().is_unlocked_for_spending() {
            warn!("Wallet is locked, new HD addresses won't be added!");
        }
        
        info!("Rescan starting");
        
        self.clear_state();

        // Then, do a sync, which will force a full rescan from the initial state
        let response = self.do_sync(true);

        info!("Rescan finished");

        response
    }

    /// Return the syncing status of the wallet
    pub fn do_scan_status(&self) -> WalletStatus {
        self.sync_status.read().unwrap().clone()
    }


    pub fn do_sync(&self, _print_updates: bool) -> Result<JsonValue, String> {
        // For doing the sync, we will connect to the Ysimple service, send our wallet file, wait for it to sync, 
        // and get it back.

        // We can only do one sync at a time because we sync blocks in serial order
        // If we allow multiple syncs, they'll all get jumbled up.
        let _lock = self.sync_lock.lock().unwrap();

        // First, we need to encrypt it first. 
        let mut password_bytes = [0u8; 32];
        let mut system_rng = OsRng;
        system_rng.fill(&mut password_bytes);
        let password = hex::encode(password_bytes);

        self.wallet.write().unwrap().encrypt(password.clone()).unwrap();

        // Get the wallet bytes
        let data: Vec<u8> = match self.do_save_to_buffer() {
            Ok(b) => b,
            Err(e) => return Err(e)
        };

        // Decrypt the wallet now, because if something goes wrong, we don't want to lock the wallet
        self.wallet.write().unwrap().remove_encryption(password.clone()).unwrap();

        let client = reqwest::blocking::ClientBuilder::new()
            .timeout(std::time::Duration::from_secs(60 * 2))
            .build().unwrap();

        let resp = client.post("https://ysimple.ycash.xyz/sync")
                    .body(data)
                    .send();

        let mut updated_wallet: Vec<u8> = vec![];
        match resp {
            Ok(mut r) => {
                if r.status().is_success() {
                    r.copy_to(&mut updated_wallet).unwrap();
                } else {
                    return Err(format!{"Response error: {:?}", r.status()});
                }
            },
            Err(e) => return Err(e.to_string())
        };

        // Now, replace the wallet
        {
            let mut new_wallet = LightWallet::read(&updated_wallet[..], &self.config).unwrap();

            // Decrypt the wallet when it comes back
            new_wallet.remove_encryption(password).unwrap();

            let mut guard = self.wallet.write().unwrap();
            std::mem::replace(&mut *guard, new_wallet);
        }

        Ok(object!{
            "result" => "success"
        })
    }

    pub fn do_send(&self, addrs: Vec<(&str, u64, Option<String>)>) -> Result<String, String> {
        if !self.wallet.read().unwrap().is_unlocked_for_spending() {
            error!("Wallet is locked");
            return Err("Wallet is locked".to_string());
        }

        info!("Creating transaction");

        let rawtx = self.wallet.write().unwrap().send_to_address(
            u32::from_str_radix(&self.config.consensus_branch_id, 16).unwrap(), 
            &self.sapling_spend, &self.sapling_output,
            addrs
        );
        
        match rawtx {
            Ok(txbytes)   => broadcast_raw_tx(&self.get_server_uri(), self.config.no_cert_verification, txbytes),
            Err(e)        => Err(format!("Error: No Tx to broadcast. Error was: {}", e))
        }
    }
}

#[cfg(test)]
pub mod tests {
    use lazy_static::lazy_static;
    use tempdir::TempDir;
    use super::{LightClient, LightClientConfig};

    lazy_static!{
        static ref TEST_SEED: String = "youth strong sweet gorilla hammer unhappy congress stamp left stereo riot salute road tag clean toilet artefact fork certain leopard entire civil degree wonder".to_string();
    }

    #[test]
    pub fn test_encrypt_decrypt() {
        let lc = super::LightClient::unconnected(TEST_SEED.to_string(), None).unwrap();

        assert!(!lc.do_export(None).is_err());
        assert!(!lc.do_new_address("z").is_err());
        assert!(!lc.do_new_address("t").is_err());
        assert_eq!(lc.do_seed_phrase().unwrap()["seed"], TEST_SEED.to_string());

        // Encrypt and Lock the wallet
        lc.wallet.write().unwrap().encrypt("password".to_string()).unwrap();
        assert!(lc.do_export(None).is_err());
        assert!(lc.do_seed_phrase().is_err());
        assert!(lc.do_new_address("t").is_err());
        assert!(lc.do_new_address("z").is_err());
        assert!(lc.do_send(vec![("z", 0, None)]).is_err());

        // Do a unlock, and make sure it all works now
        lc.wallet.write().unwrap().unlock("password".to_string()).unwrap();
        assert!(!lc.do_export(None).is_err());
        assert!(!lc.do_seed_phrase().is_err());
    }

    #[test]
    pub fn test_addresses() {
        let lc = super::LightClient::unconnected(TEST_SEED.to_string(), None).unwrap();

        // Add new z and t addresses
            
        let taddr1 = lc.do_new_address("t").unwrap()[0].as_str().unwrap().to_string();
        let taddr2 = lc.do_new_address("t").unwrap()[0].as_str().unwrap().to_string();        
        let zaddr1 = lc.do_new_address("z").unwrap()[0].as_str().unwrap().to_string();
        let zaddr2 = lc.do_new_address("z").unwrap()[0].as_str().unwrap().to_string();
        
        let addresses = lc.do_address();
        assert_eq!(addresses["z_addresses"].len(), 3);
        assert_eq!(addresses["z_addresses"][1], zaddr1);
        assert_eq!(addresses["z_addresses"][2], zaddr2);

        assert_eq!(addresses["t_addresses"].len(), 3);
        assert_eq!(addresses["t_addresses"][1], taddr1);
        assert_eq!(addresses["t_addresses"][2], taddr2);
    }

    #[test]
    pub fn test_wallet_creation() {
        // Create a new tmp director
        {
            let tmp = TempDir::new("lctest").unwrap();
            let dir_name = tmp.path().to_str().map(|s| s.to_string());

            // A lightclient to a new, empty directory works.
            let config = LightClientConfig::create_unconnected("test".to_string(), dir_name);
            let lc = LightClient::new(&config, 0).unwrap();
            let _seed = lc.do_seed_phrase().unwrap()["seed"].as_str().unwrap().to_string();
        }
    }
}