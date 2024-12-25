
mod core;

use anyhow::Result;
use clap::{Parser, Subcommand};
use kanal;
use tokio::time::{self, Duration};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use btc_lib::types::Transaction;
use std::sync::Arc;
use core::Config;
use core::Core;
use core::FeeConfig;
use core::FeeType; 
use core::Recipient;




// #[derive(Parser)]: This macro automatically generates the code needed to parse command-line arguments for the Cli struct. The Parser derive is part of the clap crate, which simplifies command-line argument parsing.
// #[command(author, version, about, long_about = None)]: This attribute specifies metadata about the CLI:
//     author: The author of the application.
//     version: The version of the application.
//     about: A short description of what the application does.
//     long_about = None: A longer description of the application, which is not provided in this case (hence None).

// #[command(subcommand)]
// command: Option<Commands>,

// #[command(subcommand)]: This attribute indicates that the Cli struct can have subcommands, and the command field will store one of those subcommands.
// command: Option<Commands>: This field holds the subcommand chosen by the user (if any). The Commands enum is defined below to list all possible subcommands. The Option type is used because the user might not provide a subcommand.

// #[arg(short, long, value_name = "FILE")]
// config: Option<PathBuf>,

// #[arg(short, long, value_name = "FILE")]: This attribute configures an argument for the Cli struct.
//     short: The argument can be specified with a short flag (e.g., -c).
//     long: The argument can also be specified with a long flag (e.g., --config).
//     value_name = "FILE": This defines how the value passed for this argument should be displayed (here it will be shown as "FILE").  

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {

    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    #[arg(short, long, value_name = "ADDRESS")]
    node: Option<String>

}

#[derive(Subcommand)]

enum Commands {

    GenerateConfig {

        #[arg(short, long, value_name = "FILE")]
        output: PathBuf,
    },
}


async fn update_utxos(core: Arc<Core>) {

    let mut interval = time::interval(Duration::from_secs(20));

    loop {

        interval.tick().await;

        if let Err(e) = core.fetch_utxos().await {

            eprintln!("failed to update UTXOS : {}", e) ;// send the error to std::err istead fo std::io usefull for debug
        }
    }


}

// In handle_transactions, we are waiting to receive
// fully formed transactions (they will be coming from the UI
// 

async fn handle_transaction(rx: kanal::AsyncReceiver<Transaction>, core: Arc<Core>) {

    while let Ok(transaction) = rx.recv().await {

        if let Err(e) = core.send_transaction(transaction).await {

            eprintln!("failed to send transaction : {}", e);
        }
    }


}


async fn run_cli(core: Arc<Core>) -> Result<()> {

    loop {


        print!("> ");

       // The flush() method is called on the standard output (stdout). 
       //It is used to ensure that any buffered data 
       //(data that is temporarily stored in memory before being written out) 
       // is written to the output device immediately.
       // In Rust, standard output (like many other I/O operations) is 
       // often buffered for efficiency. Calling flush() ensures that the contents of the buffer are actually written to the console, even if the buffer hasn't been automatically flushed yet (for example, if the program terminates prematurely).
        
        //We print the prompt, and then we start reading commands. Notice how we need
        //to flush the standard output after using the print!() macro. While the println!()
        //macro flushes by default, print!() doesn’t.

        io::stdout().flush()?;

        let mut input = String::new();

        io::stdin().read_line(&mut input)?;

        let parts: Vec<&str> = input.trim().split_whitespace().collect();

        if parts.is_empty() {

            continue;
        }

        match parts[0] {

            "balance" => {

                // process balance

                println!("Current balance: {} satoshis", core.get_balance());


            }

            "send" => {

                // process send

                if parts.len() != 3 {
                    
                    println!("Usage: send <recipient> <amount>");
                    
                    continue;

                }

                let recipient = parts[1];

                let amount:u64 = parts[2].parse()?; // why integer , amount can be floating point

                let recipient_key = core    
                    .config
                    .contacts
                    .iter()
                    .find(|r| r.name == recipient)
                    .ok_or_else( || {
                        anyhow::anyhow!("Recipeint not found")
                    })?
                    .load()?
                    .key;
                
                if let Err(e) = core.fetch_utxos().await {

                    println!("failed to fetch utxos: {e}");
                };

                let transaction = core.create_transaction(&recipient_key, amount).await?;

                core.tx_sender.send(transaction).await?;

                core.fetch_utxos().await?;
            }

            "exit" => break,

            _  => {

                println!("Unknown command");
            }
        }


    }


    Ok(())

}

fn generate_dummy_config(path: &PathBuf) -> Result<()> {

    let dummy_config = Config {

        my_keys: vec![],

        contacts: vec![
            Recipient {

                name: "Alice".to_string(),

                key: PathBuf::from("alice.pub.pem")
            },
            Recipient {

                name: "Bob".to_string(),

                key: PathBuf::from("bob.pub.pem"),
             },

        ],

        default_node: "127.0.0.1:9000".to_string(),
        
        fee_config: FeeConfig {

            fee_type: FeeType::Percent,

            value: 0.1,
        }


    };

    let config_str = toml::to_string_pretty(&dummy_config)?;
    
    std::fs::write(path, config_str)?;

    println!("Dummy config generated at : {}", path.display());

    Ok(())
}


#[tokio::main]

async fn main() -> Result<()> {

    let cli = Cli::parse();

    match &cli.command {

        Some(Commands::GenerateConfig { output }) => {

            return generate_dummy_config(output);


        }

        None => {}
    }

    let config_path = cli.config.unwrap_or_else( || PathBuf::from("wallet_config.toml"));

    let mut core = Core::load(config_path.clone())?;

    if let Some(node) = cli.node {
        
        core.config.default_node = node;


    }


    let (tx_sender, tx_receiver) = kanal::bounded(10);

    core.tx_sender = tx_sender.clone_async();

    let core = Arc::new(core);

    tokio::spawn(update_utxos(core.clone()));

    tokio::spawn(handle_transaction(tx_receiver.clone_async(), core.clone()));

    run_cli(core).await?;


    Ok(())
}






