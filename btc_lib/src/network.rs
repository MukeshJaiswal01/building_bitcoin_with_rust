use serde::{Deserialize, Serialize};
use crate::crypto::PublicKey;
use crate::types::{Block, Transaction, TransactionOutput};
use std::io::{Error as IoError, Read, Write};

use tokio::io::{

    AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Message {

    // Fetch all UTXO's belonging to a public key
    FetchUTXOs(PublicKey),

    // UTXO's belonging to a public key. Bool determines if marked 
    UTXOs(Vec<(TransactionOutput, bool)>), 

    // send the transaction to the network
    SubmitTransaction(Transaction),

    // Broadcast a new transaction to other nodes
    NewTransaction(Transaction),

    // Ask the node to 
    // prepare the optimal block template with the coinbase transaction paying the specified public key
    FetchTemplate(PublicKey),

    // The template
    Template(Block),

    // Ask the node to validate a block template.
    // this is to prevent node from mining an invalid block{ 
    //  Ex- another block has been found in meantime
    //      or if the transaction has been removed from the mempool
    // }
    ValidateTemplate(Block),

    // If template is valid
    TemplateValidity(bool),

    // Submit the mined block to a node
    SubmitTemplate(Block), 

    /// Ask a node to report to all other nodes it knows about
    DiscoverNodes,

    // This is the response to DiscoverNodes
    NodeList(Vec<String>), 

    // Ask a node what's the highest block it knows about in comparison to the local blockchain
    AskDifference(u32),

    // this is the response to AskDifference
    Difference(i32),

    // Ask a node to send a block with specified height 
    FetchBlock(usize),

    // Broadcast a new block to other nodes
    NewBlock(Block),


}


// we are going to use length-prefixed encoding for message and we are going to use CBOR for serialization
// first we send the message lenght then the actual message so that receiver will get 
// to know how much of message to expect

impl Message {

    pub fn encode(&self) -> Result< Vec<u8>, ciborium::ser::Error<IoError>> {

        let mut bytes = Vec::new();
        
        ciborium::into_writer(self, &mut bytes)?;

        Ok(bytes)


    }

    pub fn decode(data: &[u8],) -> Result<Self, ciborium::de::Error<IoError>> {

        ciborium::from_reader(data)
    }

    pub fn send(&self, stream: &mut impl Write,) -> Result<(), ciborium::ser::Error<IoError>> {


        let bytes  = self.encode()?;

        let len = bytes.len() as u64;

        stream.write_all(&len.to_be_bytes())?;

        stream.write_all(&bytes)?;

        Ok(())

    }

    pub fn receive(stream: &mut impl Read,) -> Result<Self, ciborium::de::Error<IoError>> {

        let mut len_bytes = [0u8; 8];  // what if it does not matches the expected length

        stream.read_exact(&mut len_bytes)?;

        let len = u64::from_be_bytes(len_bytes) as usize;

        let mut data = vec![0u8; len];

        stream.read_exact(&mut data)?;

        Self::decode(&data)




    }

//     Pinning in Rust

// In Rust, pinning is the concept of guaranteeing that a value's memory location cannot be moved after it has been "pinned." Pinning is important for types that rely on their memory address remaining fixed, such as self-referential structs or types used with async programming.

// In Rust, types that are "pinned" can be thought of as being "anchored" to a specific memory location, which prevents them from being moved. This is essential in certain cases, for example, when working with asynchronous tasks or when the type holds references to itself or depends on its location in memory.
// The Unpin Trait

// The Unpin trait is used to indicate that a type can be safely moved after it has been pinned. If a type implements Unpin, it means that the compiler can move the value in memory without violating the constraints of pinning


    pub async fn send_async(&self, stream: &mut (impl AsyncWrite + Unpin)) -> Result<(), ciborium::ser::Error<IoError>> {


        let bytes = self.encode()?;

        let len = bytes.len() as u64;

        stream.write_all(&len.to_be_bytes()).await?;

        stream.write_all(&bytes).await?;

        Ok(())



    }


    

    pub async  fn receive_async(stream: &mut (impl AsyncRead + Unpin),) -> Result<Self, ciborium::de::Error<IoError>> {
        

        let mut len_bytes = [0u8; 8];   // why only eight bytes

        stream.read_exact(&mut len_bytes).await ?;

        let len = u64::from_be_bytes(len_bytes) as usize;

        let mut data = vec![0u8; len];   
        
        stream.read_exact(&mut data ).await ?;

        Self::decode(&data)



    }
}


