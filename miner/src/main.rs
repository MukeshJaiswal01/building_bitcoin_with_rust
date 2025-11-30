use std::env;
use std::process::exit;
use btc_lib::crypto::PublicKey;
use btc_lib::types::Block;
use btc_lib::util::Saveable;
use btc_lib::network::Message;
use std::thread;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

use tokio::net::TcpStream;
use clap::Parser;
use anyhow::{anyhow, Result}; // for Standard error handling
use std::result::Result::Ok;

use std::sync:: {

    atomic::{AtomicBool, Ordering}, Arc,
};

// parser is a component responsible for interpreting 
// and processing the input provided by the user via the command lin

#[derive(Parser)]
#[command(author, version, about, long_about = None)]

struct  Cli {

    #[arg(short, long)]
    address:String,

    #[arg(short, long)]
    public_key_file: String,
}


// Mutex<T> is a synchronization primitive 
// that provides safe access to shared data in a concurrent context. 
// It ensures that only one thread at a time can access the TcpStream. T
// his is important because multiple threads may need to perform read/write operations on the stream, and the Mutex ensures thread-safety by locking access to it

// Arc<T>: This is a thread-safe reference-counted smart pointer. 
// It allows multiple threads to own the same data (Option<Block> in this case),
//  and the data will remain alive as long as there is at least one Arc pointing to it

// AtomicBool: This is a type from the std::sync::atomic module, 
// which represents a boolean value that can be shared safely between threads without needing locks. 
// It supports atomic operations like reading, setting, and toggling the value


// flume::Sender<T>: This is a channel type from the flume crate, 
// which is an alternative to Rust’s standard library mpsc channels. It allows for safe communication between threads by sending messages from one thread to another.


struct Miner {

    public_key: PublicKey,

    stream: Mutex<TcpStream>,

    current_template: Arc<std::sync::Mutex<Option<Block>>>,

    mining: Arc<AtomicBool>,

    mined_block_sender: flume::Sender<Block>,

    mined_block_receiver: flume::Receiver<Block>,

}

impl Miner {


    async fn new(address: String, public_key: PublicKey) -> Result<Self> {

        let stream = TcpStream::connect(&address).await?;

        let (mined_block_sender, mined_block_receiver) = flume::unbounded();

        println!("making changes");

        Ok(Self {

            public_key,
            
            stream: Mutex::new(stream),

            current_template: Arc::new(std::sync::Mutex::new(None,)),

            mining: Arc::new(AtomicBool::new(false)),

            mined_block_sender,

            mined_block_receiver,

        })


    }

    async fn run(&self) -> Result<()> {

        /// create new thread 
        self.spawn_mining_thread();


        /// this line creates a periodic timer using tokio::time::interval. The template_interval will fire once every 5 seconds

        let mut template_interval = interval(Duration::from_secs(5));

        loop {

            // is used to receive mined blocks from the receiver asynchronously

            let receiver_clone = self.mined_block_receiver.clone();
             

           // The tokio::select! macro is used to await multiple
           // asynchronous operations at the same time and proceed with whichever one completes first. It allows for more efficient handling of multiple asynchronous tasks in parallel.
           
            tokio::select! {


                // This case awaits the next tick from template_interval (the periodic timer we created earlier). The _ means that the value returned by tick() is ignored (we just care that it completes).
                // Once the interval ticks (every 5 seconds), it asynchronously calls fetch_and_validate_template and awaits its result.

                _ = template_interval.tick() => {

                    self.fetch_and_validate_template().await?;
                }


                // This case handles receiving a mined_block 
                // from the receiver_clone. The recv_async() function is used to asynchronously receive a value from the receiver.
                // When a new mined block is received, the code attempts to submit the block by calling submit_block(mined_block).await?

                Ok(mined_block) = receiver_clone.recv_async() => {

                    self.submit_block(mined_block).await?;
                }


            }
        }


    }

    // A JoinHandle is essentially a handle or reference to a thread that has been spawned. You can use it to:

    // Wait for the thread to finish (join the thread).
    // Obtain the result of the thread's execution (if the thread returns a value).
    // Handle any errors that occurred in the thread
    // its OS thread
    // @note Rust threading only uses OS threads

    // Green thread: threads are scheduled and managed by a user-level runtime
    // meaning the programmer or runtime system decides when to give up control of the CPU, which can be less efficient but more lightweight compared to OS-level threads
    
    fn spawn_mining_thread(&self) -> thread::JoinHandle<()> {

        let template = self.current_template.clone();

        let mining = self.mining.clone();

        let sender = self.mined_block_sender.clone();

        thread::spawn( move || loop {



            // In Rust, atomic types and operations are provided by the std::sync::atomic module. 
            // These operations are low-level, providing a way to work with data shared between threads without locking. However, when using atomic operations, the compiler and CPU are free to reorder operations for performance reasons, which could lead to unexpected results in certain cases. Memory ordering is the mechanism used to control how these operations are ordered.
             
           //  Ordering::Relaxed allows atomic operations to occur without enforcing synchronization between threads, meaning that the compiler and CPU can freely reorder operations
           // others Ordering::SeqCst .... (safest one)


              if mining.load(Ordering::Relaxed) {

                if let Some(mut block) = template.lock().unwrap().clone() {

                    println!("Mining block with target: {}", block.header.target);

                    if block.header.mine(2_000_000) {

                        println!("block mined : {}", block.hash());

                        sender.send(block).expect("failed to send mined block");

                        mining.store(false, Ordering::Relaxed);
                    }


                }
            }


           //  you are suggesting to the operating system's thread scheduler that the current thread is willing to give up its remaining time slice or CPU time and allow other threads to run. It does not suspend or stop the current thread; it just allows the system to potentially switch to another thread that might be waiting to run

            thread::yield_now();


        })


       // unlockiing of current mutex, it will be unlocked when the  call inside the loop is over, every iteration lock and unlock happen
       // The Mutex will only be released when the thread that owns 
       //the lock calls .unlock() (in case of explicit unlocking) or when the lock guard (returned by .lock()) goes out of scope. This is how Rust's ownership and borrowing rules ensure proper synchronization




    }

    async fn fetch_and_validate_template(&self) -> Result<()> {

        if !self.mining.load(Ordering::Relaxed) {

            self.fetch_template().await?;

        } else {

            self.validate_template().await?;
        }

        Ok(())




    }


    async fn fetch_template(&self) -> Result<()> {

        println!("fetching new template");

        let message = Message::FetchTemplate(self.public_key.clone());

        let mut stream_lock = self.stream.lock().await;

        message.send_async(&mut *stream_lock).await?;
       
       // dropping the lock so that to prevent deadlock
        drop(stream_lock);

        let mut stream_lock = self.stream.lock().await;

        match Message::receive_async(&mut *stream_lock).await? {

            Message::Template(template) => {

                drop(stream_lock);

                println!("Received new template with target: {}", template.header.target);

                *self.current_template.lock().unwrap() = Some(template);

                self.mining.store(true, Ordering::Relaxed);

                Ok(())
            }

            _ => Err(anyhow!("unexpected message received when fethching")),
        }





    }

    async fn validate_template(&self) -> Result<()> {

        if let Some(template) = self.current_template.lock().unwrap().clone() {

            let message = Message::ValidateTemplate(template);

            let mut stream_lock = self.stream.lock().await;
            
            message.send_async(&mut *stream_lock).await?;

            drop(stream_lock);

            let mut stream_lock = self.stream.lock().await;

            match Message::receive_async(&mut *stream_lock).await? {


                Message::TemplateValidity(valid) => {

                    drop(stream_lock);

                    if !valid {

                        println!("Current template is no longer valid");

                        self.mining.store(false, Ordering::Relaxed);

                    } else {

                        println!("Current template is still valid");
                    }

                    Ok(())
                }

                _ => Err(anyhow!("unexpected message received when validating template")),
            }

           

        }   else {

            Ok(())
        }



    }

    async fn submit_block(&self, block: Block) -> Result<()> {

        println!("Submitting mined block");

        let message = Message::SubmitTemplate(block);

        let mut stream_lock = self.stream.lock().await;

        message.send_async(&mut *stream_lock).await?;

        self.mining.store(false, Ordering::Relaxed);

        Ok(())


    }



}


#[tokio::main]

async fn main() -> Result<()> {


    let cli = Cli::parse();

    let public_key = PublicKey::load_from_file(&cli.public_key_file)
        
        .map_err(|e| {

            anyhow!("Error reading public key: {}", e)

        })?;

    let miner = Miner::new(cli.address, public_key).await?;

    miner.run().await

    
}












































// fn usage() -> ! {

//     eprintln!( "Usage: {} <address> <public_key_file>", env::args().next().unwrap());

//     exit(1);
    
// }

// #[tokio::main]
// async fn main () {

//     let address = match env::args().nth(1) {

//         Some(address) => address,
//         None => usage(),
//     };

//     // let public_key_file = match env::args().nth(2) {

//     //     Some(public_key_file) => public_key_file,
//     //     None => usage(),
//     // };

//     // let Ok(public_key) = 

//     //     PublicKey::load_from_file(&public_key_file)

//     //     else {

//     //         eprintln!("Error reading public key from the file {}", public_key_file);
//     //         exit(1)
//     //     };

//     // println!("Connecting to {address} to mine with {public_key:?}");


//     // // Try connecting to the node
    

//     // let mut stream = match TcpStream::connect(&address).await {

//     //     Ok(stream) => stream,

//     //     Err(e) => {

//     //         eprintln!("failed to connect to server {}", e)

//     //         exit(1);
//     //     }


//     // };

//     // // Ask the node for work

//     // println!("requesting work from {address}");

//     // let message = Message::FetchTemplate(public_key);

//     // message.send(&mut stream);





// }




// fn main() {
   


//     // parse block path ans steps count from first ans second argument

//     let (path, steps) = if let (Some(arg1), Some(arg2)) = 
//         ((env::args().nth(1)), env::args().nth(2)) {

//             (arg1, arg2)

//         } else {

//             eprintln!("usage: miner <block_file> <steps>");
            
//             exit(1);
//         };

    
//     // parse steps count

//     let steps: usize = if let Ok(s @ 1..=usize::MAX) = steps.parse() {   // @ bind the s value to that range

//         s

//     } else {

//         eprint!("<steps> should be a positive integer");

//         exit(1);
//     };

    
//     /// load block from fiel 
    
//     let og_block = Block::load_from_file(path).expect("failed to load the block");

//     let mut block = og_block.clone();

//     while !block.header.mine(steps) {

//         println!("mining .... ");
        
//     }

//     // print original block and its hash

//     println!("original: {:#?}", og_block);
//     println!("hash: {}", og_block.header.hash());

//     // print mined block and its hash

//     println!("final: {:#?}", block);
//     println!("hash: {}", block.header.hash());

// }




