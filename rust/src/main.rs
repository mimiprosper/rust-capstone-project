use bitcoincore_rpc::bitcoin::{Address, Amount};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use std::fs::File;
use std::io::Write;
use std::path::Path;

const RPC_URL: &str = "http://127.0.0.1:18443";
const RPC_USER: &str = "alice";
const RPC_PASS: &str = "password";


/// Main function that performs the following operations:
/// 1. Connects to a Bitcoin Core RPC server.
/// 2. Creates "Miner" and "Trader" wallets if they do not exist.
/// 3. Mines 101 blocks to mature the coinbase transaction.
/// 4. Sends 20 BTC from the "Miner" wallet to the "Trader" wallet.
/// 5. Mines an additional block to confirm the transaction.
/// 6. Retrieves and outputs transaction details, including miner and trader addresses, 
///    change address, transaction fee, block height, and block hash.
/// 7. Writes transaction details to an output file named "out.txt".
/// 
/// Returns:
/// - A bitcoincore_rpc::Result indicating success or failure of the operations.
fn main() -> bitcoincore_rpc::Result<()> {
    // Connect to RPC (base connection without wallet)
    let rpc = Client::new(
        RPC_URL,
        Auth::UserPass(RPC_USER.to_owned(), RPC_PASS.to_owned()),
    )?;

    // Create wallets if they don't exist
    let wallets = rpc.list_wallets()?;
    if !wallets.contains(&"Miner".to_string()) {
        rpc.create_wallet("Miner", None, None, None, None)?;
    }
    if !wallets.contains(&"Trader".to_string()) {
        rpc.create_wallet("Trader", None, None, None, None)?;
    }

    // Create wallet-specific clients
    let miner = Client::new(
        &format!("{}/wallet/Miner", RPC_URL),
        Auth::UserPass(RPC_USER.to_owned(), RPC_PASS.to_owned()),
    )?;

    let trader = Client::new(
        &format!("{}/wallet/Trader", RPC_URL),
        Auth::UserPass(RPC_USER.to_owned(), RPC_PASS.to_owned()),
    )?;

    // Generate mining address and mine blocks
    let mining_address = miner.get_new_address(Some("Mining Reward"), None)?.assume_checked();
    
    // Mine 101 blocks to mature the coinbase transaction
    rpc.generate_to_address(101, &mining_address)?;
    
    // Get miner balance
    let miner_balance = miner.get_balance(None, None)?;
    println!("Miner balance: {} BTC", miner_balance.to_btc());

    // Generate trader address
    let trader_address = trader.get_new_address(Some("Received"), None)?.assume_checked();

    // Send 20 BTC from Miner to Trader
    let txid = miner.send_to_address(
        &trader_address,
        Amount::from_btc(20.0)?,
        None,
        None,
        None,
        None,
        None,
        None,
    )?;

    // Mine 1 block to confirm the transaction
    let block_hashes = rpc.generate_to_address(1, &mining_address)?;
    let block_hash = block_hashes[0];
    
    // Get block height
    let block_height = rpc.get_block_count()?;

    // Get transaction details
    let tx = miner.get_transaction(&txid, None)?;
    let decoded = miner.get_raw_transaction(&txid, None)?;
    
    // Find change output (output that's not to trader)
    let change_output = decoded.output.iter()
        .find(|o| {
            let script = &o.script_pubkey;
            let addr = Address::from_script(script, bitcoincore_rpc::bitcoin::Network::Regtest)
                .unwrap_or_else(|_| panic!("Failed to convert script to address"));
            addr != trader_address
        })
        .expect("Change output not found");
    
    let change_address = Address::from_script(
        &change_output.script_pubkey,
        bitcoincore_rpc::bitcoin::Network::Regtest
    ).expect("Failed to convert change script to address");

    // Calculate fee (absolute value)
    let fee = tx.fee.unwrap().to_btc().abs();

    // Write to out.txt in the project root directory
    let out_path = Path::new("../out.txt");
    let mut file = File::create(out_path)?;
    writeln!(file, "{}", txid)?;
    writeln!(file, "{}", mining_address)?;
    writeln!(file, "50")?; // Miner's input amount (block reward)
    writeln!(file, "{}", trader_address)?;
    writeln!(file, "20")?; // Trader's output amount
    writeln!(file, "{}", change_address)?;
    writeln!(file, "{}", change_output.value.to_btc())?;
    writeln!(file, "{}", fee)?;
    writeln!(file, "{}", block_height)?;
    writeln!(file, "{}", block_hash)?;

    println!("Transaction details written to out.txt successfully");

    Ok(())
}