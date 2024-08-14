use dotenv::dotenv;
use helius::types::*;
use helius::Helius;
use rand::Rng;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use solana_sdk::{signature::Keypair, system_instruction};
use std::env;
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::str::FromStr;
use tokio;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let api_key = env::var("API_KEY").expect("No API_KEY provided");
    let keystring = env::var("PRIVATE_KEY").expect("No PRIVATE_KEY provided");
    let helius = Helius::new(&api_key, Cluster::MainnetBeta).unwrap();

    let keybytes: Vec<u8> = keystring
        .trim_matches(|p| p == '[' || p == ']')
        .split(',')
        .map(str::trim)
        .filter_map(|s| s.parse().ok())
        .collect();

    let wallet = Keypair::from_bytes(&keybytes).unwrap();
    let from_pubkey = wallet.pubkey();

    smart_transactions(helius, from_pubkey, wallet)
        .await
        .unwrap();
}

async fn smart_transactions(
    helius: Helius,
    from_pubkey: Pubkey,
    wallet: Keypair,
) -> io::Result<()> {
    let entries = get_addresses().unwrap();
    let batch_size = 20;
    let mut batch_num = 1;

    for chunk in entries.chunks(batch_size) {
        let mut instructions: Vec<Instruction> = Vec::new();

        for entry in chunk {
            let to_pubkey = pubkey::Pubkey::from_str(&entry.address).unwrap();
            let amt = entry.amount;

            let ix = system_instruction::transfer(&from_pubkey, &to_pubkey, amt);
            instructions.push(ix);
        }

        let config = SmartTransactionConfig {
            create_config: CreateSmartTransactionConfig {
                instructions,
                signers: vec![&wallet],
                lookup_tables: None,
                fee_payer: Some(&wallet),
            },
            send_options: RpcSendTransactionConfig {
                skip_preflight: true,
                preflight_commitment: None,
                encoding: None,
                max_retries: Some(2),
                min_context_slot: None,
            },
        };

        match helius.send_smart_transaction(config).await {
            Ok(res) => {
                println!(
                    "Transaction successful for batch {:?}: {:?}",
                    batch_num, res
                );
            }
            Err(e) => {
                println!("Error in batch {:?}: {:?}", batch_num, e);
            }
        }

        batch_num += 1;
    }

    Ok(())
}

struct Entry {
    address: String,
    amount: u64,
}

//Creates a file with 1000 random addresses and amounts
// fn create_address_file() -> io::Result<()> {
//     let mut file = File::create("payment_addresses.txt")?;
//     let mut rng = rand::thread_rng();

//     for _ in 0..1000 {
//         let keypair = Keypair::new();
//         let address = keypair.pubkey();

//         let amount = rng.gen_range(10000..=20000);

//         writeln!(file, "{},{}", address, amount)?;
//     }

//     Ok(())
// }

fn process_line(line: &str) -> io::Result<Entry> {
    let parts: Vec<&str> = line.split(',').collect();
    let address = parts[0].to_string();
    let amount = parts[1].trim().parse().unwrap();

    Ok(Entry { address, amount })
}

fn get_addresses() -> io::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    let file = File::open("payment_addresses.txt")?;
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        let entry = process_line(&line)?;
        entries.push(entry);
    }

    Ok(entries)
}
