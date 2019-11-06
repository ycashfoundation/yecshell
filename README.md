## Yecwallet CLI - A command line ZecWallet light client. 

`yecwallet-cli` is a command line ZecWallet light client. To use it, download the latest binary from the releases page and run `./yecwallet-cli`

This will launch the interactive prompt. Type `help` to get a list of commands

## Running in non-interactive mode:
You can also run `yecwallet-cli` in non-interactive mode by passing the command you want to run as an argument. For example, `yecwallet-cli addresses` will list all wallet addresses and exit. 
Run `yecwallet-cli help` to see a list of all commands. 

## Privacy 
* While all the keys and transaction detection happens on the client, the server can learn what blocks contain your shielded transactions.
* The server also learns other metadata about you like your ip address etc...
* Also remember that t-addresses don't provide any privacy protection.

## Notes:
* The wallet connects to the mainnet by default. To connect to testnet, please pass `--server https://lightd-test.zecwallet.co:443`
* If you want to run your own server, please see [zecwallet lightwalletd](https://github.com/adityapk00/lightwalletd), and then run `./yecwallet-cli --server http://127.0.0.1:9067`. You might also need to pass `--dangerous` if you are using a self-signed  TLS certificate.

* The log file is in `~/.zcash/zecwallet-light-wallet.debug.log`. Wallet is stored in `~/.zcash/zecwallet-light-wallet.dat`

### Note Management
Yecwallet-CLI does automatic note and utxo management, which means it doesn't allow you to manually select which address to send outgoing transactions from. It follows these principles:
* Defaults to sending shielded transactions, even if you're sending to a transparent address
* Sapling funds need at least 4 confirmations before they can be spent
* Can select funds from multiple shielded addresses in the same transaction
* Will automatically shield your transparent funds at the first opportunity
    * When sending an outgoing transaction to a shielded address, Yecwallet-CLI can decide to use the transaction to additionally shield your transparent funds (i.e., send your transparent funds to your own shielded address in the same transaction)

## Compiling from source

#### Pre-requisites
* Rust v1.37 or higher.
    * Run `rustup update` to get the latest version of Rust if you already have it installed

```
git clone https://github.com/adityapk00/zecwallet-light-cli.git
cargo build --release
./target/release/yecwallet-cli
```

## Options
Here are some CLI arguments you can pass to `yecwallet-cli`. Please run `yecwallet-cli --help` for the full list. 

* `--server`: Connect to a custom zecwallet lightwalletd server. 
    * Example: `./yecwallet-cli --server 127.0.0.1:9067`
* `--seed`: Restore a wallet from a seed phrase. Note that this will fail if there is an existing wallet. Delete (or move) any existing wallet to restore from the 24-word seed phrase
    * Example: `./yecwallet-cli --seed "twenty four words seed phrase"`
 * `--recover`: Attempt to recover the seed phrase from a corrupted wallet
 
