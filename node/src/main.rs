use anyhow::Result;
use argh::FromArgs; // handling command line arguments
use dashmap::DashMap; // Provides a fast HashMap that is thread-safe and has interior mutability
use static_init::dynamic; // creating global variable 
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use btc_lib::types::Blockchain;





use std::path::Path;  // its a struct used for handling filesystem paths in a platform-independent way (different os)


mod handler;
mod util;


// The static_init::dynamic you're referring to seems to be part of the static_init crate, which is used for static initialization in Rust. The static_init::dynamic macro is specifically used to create a 
// dynamically initialized static variable. This crate is designed for situations where you need to perform complex initialization (like computations) that you can’t do at compile-time but still want to store the result in a static variable.


// In Rust, RwLock (Read-Write Lock) is a synchronization primitive that allows multiple readers to access data concurrently, but only one writer can access the data at a time. It's part of the std::sync module and is used when you want to manage access to shared data in a multi-threaded environment.

//     Readers can access the data concurrently, meaning if multiple threads are only reading the data, they can do so without blocking each other.
//     Writers have exclusive access to the data, meaning while a thread is writing, no other thread can read or write the data

#[dynamic]
pub static BLOCKCHAIN:RwLock<Blockchain> =  RwLock::new(Blockchain::new());  // Rwlock provide interior mutability

// Node pool

#[dynamic]
pub static NODES: DashMap<String, TcpStream> = DashMap::new();




#[derive(FromArgs)]

/// A toy blockchain node
struct Args {

    #[argh(option, default = "9000")]
    /// port number
    port: u16,

   #[argh(option, default = "String::from(\"./blockchain.cbor\")" )]
    /// blockchain file locatioin  --> /// doc comment
    blockchain_file: String,    

    #[argh(positional)]
    // address of initial nodes
    nodes: Vec<String>,
}


#[tokio::main]  // setup asnyc runtime for the main()
async fn main() -> Result<()> {


// ● A port to listen to
// ● Path to store/load the blockchain from
// ● A list of other nodes to connect to and communicate with

    // parse the command line arguments

    let args:Args = argh::from_env();

    // Access the parsed arguments
    let port = args.port;

    let blockchain_file = args.blockchain_file;

    let nodes = args.nodes;

    if Path::new(&blockchain_file).exists() {


        util::load_blockchain(&blockchain_file).await?;

        println!("total amount of known nodes: {}", NODES.len());

    } else {

        println!("blockchain file does not exist");

        util::populate_connections(&nodes).await?;

        println!("total amount of known nodes: {}",NODES.len());

        if nodes.is_empty() {

            println!("no initial nodes provided, starting as a seed node");


        } else {

            let (longest_name, longest_count) = util::find_longest_chain_node().await?;

            // request the blockchain from the node with the longest blockchain

            util::download_blockchain(&longest_name, longest_count).await?;

            println!("blockchain download from {}", longest_name);

            {   // recalculate the utxo

                let mut blockchain = BLOCKCHAIN.write().await;
                
                blockchain.rebuild_utxos();

            }


            {    // try to adjust difficulty

                let mut blockchain = BLOCKCHAIN.write().await;

                blockchain.try_adjust_target();

            }




        }



        
    }


    // Handling the requests

    // start the TCP listener on 0.0.0.0:port

    let addr = format!("0.0.0.0:{}", port);

    let listener = TcpListener::bind(&addr).await?;

    println!("listening on {}", addr);

    // start a task to periodically cleanup to mempool,
        // normally you would want to keep and join the hanle

    tokio::spawn(util::cleanup());

    // and a task to periodically save the blockchain

     tokio::spawn(util::save(blockchain_file.clone()));

    loop {

        let (socket, _) = listener.accept().await?;

        tokio::spawn(handler::handle_connection(socket));


    }







    
}








