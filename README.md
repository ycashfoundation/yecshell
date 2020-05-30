## YecShell - A command line Ycash light client.

YecShell is a command line Ycash light client. To use it, download the latest binary from the releases page and run `./yecshell`

This will launch the interactive prompt. Type `help` to get a list of commands

## Running in non-interactive mode:
You can also run `yecshell` in non-interactive mode by passing the command you want to run as an argument. For example, `yecshell addresses` will list all wallet addresses and exit.
Run `yecshell help` to see a list of all commands.

## Privacy
* While all the keys and transaction detection happens on the client, the server can learn what blocks contain your shielded transactions.
* The server also learns other metadata about you like your ip address etc . . .
* Also remember that t-addresses don't provide any privacy protection.

## Notes:
* The wallet connects to the mainnet by default `--server https://lightwalletd.ycash.xyz:443`
* If you want to run your own server, please see [lightwalletd](https://github.com/ycashfoundation/lightwalletd, and then run `./yecshell --server http://127.0.0.1:9067`. You might also need to pass `--dangerous` if you are using a self-signed  TLS certificate.
* For Linux, the log file is in `~/.ycash/lite_debug.log` and the wallet is stored in `~/.ycash/lite_wallet.dat`. For MacOS, the enclosing directory
is `/Users/<username>/Library/Application Support/Ycash`. For Windows, the enclosing directory is `%HOMEPATH%\AppData\Roaming\Zcash`.
* Because YecShell and YecLite share the same wallet file and log file, do not
run YecShell and YecLite simultaneously on the same computer. (You can switch back and forth between the two, but do not run them simultaneously.)



### Note Management
YecShell does automatic note and utxo management, which means it doesn't allow you to manually select which address to send outgoing transactions from. It follows these principles:
* Defaults to sending shielded transactions, even if you're sending to a transparent address
* Sapling funds need at least 5 confirmations before they can be spent
* Can select funds from multiple shielded addresses in the same transaction
* Will automatically shield your transparent funds at the first opportunity
    * When sending an outgoing transaction to a shielded address, YecShell can decide to use the transaction to additionally shield your transparent funds (i.e., send your transparent funds to your own shielded address in the same transaction)

## Compiling from source

#### Pre-requisites
* Rust v1.37 or higher.
    * Run `rustup update` to get the latest version of Rust if you already have it installed

```
git clone https://github.com/ycashfoundation/yecwallet-light-cli.git
cargo build --release
./target/release/yecshell
```

## Options
Here are some CLI arguments you can pass to `yecshell`. Please run `yecshell --help` for the full list.

* `--server`: Connect to a custom Ycash lightwalletd server.
    * Example: `./yecshell --server 127.0.0.1:9067`
* `--seed`: Restore a wallet from a seed phrase. Note that this will fail if there is an existing wallet. Delete (or move) any existing wallet to restore from the 24-word seed phrase
    * Example: `./yecshell --seed "twenty four words seed phrase"`
 * `--recover`: Attempt to recover the seed phrase from a corrupted wallet

 ## Capabilities

The following commands are available from YecShell. They can be run from an interactive session or in conjuction with a call to YecShell with the format `./yecshell <command>`.

- `save` - Save wallet file to disk
- `height` - Get the latest block height that the wallet is at
- `quit` - Quit the lightwallet, saving state to disk
- `lock` - Lock a wallet that's been temporarily unlocked
- `sync` - Download CompactBlocks and sync to the server
- `export` - Export private key for wallet addresses
- `send` - Send YEC to the given address/es
- `help` - Lists all available commands
- `notes` - List all sapling notes and utxos in the wallet
- `encryptionstatus` - Check if the wallet is encrypted and if it is locked
- `syncstatus` - Get the sync status of the wallet
- `decrypt` - Completely remove wallet encryption
- `balance` - Show the current YEC balance in the wallet
- `list` - List all transactions in the wallet
- `seed` - Display the seed phrase
- `rescan` - Rescan the wallet, downloading and scanning all blocks and transactions
- `addresses` - List all addresses in the wallet
- `encrypt` - Encrypt the wallet with a password
- `unlock` - Unlock wallet encryption for spending
- `info` - Get the lightwalletd server's info
- `clear` - Clear the wallet state, rolling back the wallet to an empty state.
- `new z` or `new t` - Create a new address in this wallet

