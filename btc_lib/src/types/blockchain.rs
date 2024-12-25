use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use super::{Block, Transaction, TransactionOutput};
use crate::error::{BtcError, Result};
use crate::sha256::Hash;
use crate::util::MerkleRoot;
use crate::U256;
use std::collections::{HashMap, HashSet};
use crate::util::Saveable;

use std::io:: {

    Error as IoError, ErrorKind as IoErrorKind, Read, Result as IoResult, Write,

};


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Blockchain {


    //HashMap, with the Hash of the transaction output being used as
   // the key type:

    utxos: HashMap<Hash, (bool, TransactionOutput)>,

    blocks: Vec<Block>,

    target: U256,

    #[serde(default, skip_serializing)]
    mempool: Vec<(DateTime<Utc>, Transaction)>,
}


impl Saveable for Blockchain {


    fn load<I: Read>(reader: I) -> IoResult<Self> {

        ciborium::de::from_reader(reader).map_err(|_| {

            IoError::new(IoErrorKind::InvalidData, "failed to deserialize the blockchain")
        })
    } 


    fn save<O: Write>(&self, writer: O) -> IoResult<()> {


        ciborium::ser::into_writer(self, writer).map_err(|_| {

            IoError::new(IoErrorKind::InvalidData, "failed to serialize blockchain")
        })
    }
}




impl Blockchain  {

    pub fn new() -> Self {

        Blockchain{

            utxos: HashMap::new(),
            
            blocks: vec![],

            target: crate::MIN_TARGET,

            mempool: vec![],
            
            }
    }


    // Rebuild Utxo set from the block chain 

    pub fn rebuild_utxos(&mut self) {


        for block in &self.blocks {

            for transaction in &block.transactions {

                for input in &transaction.inputs {


                    self.utxos.remove(&input.prev_transaction_output_hash);
                }

                for output in  transaction.outputs.iter() {

                    self.utxos.insert(transaction.hash(), (false, output.clone()));
                }
            }
        }
    }

    pub fn add_block(&mut self, block: Block) -> Result<()> {

        // check if the block is valid

        if self.blocks.is_empty() {

            // if this is the first block, check if the block's prev hash is all zeros

            if block.header.prev_block_hash != Hash::zero() {

                println!("zero hash");
                return Err(BtcError::InvalidBlock);
            }

        } else {

            // if this is not the first block , check if the block's prev_block_hash is the hash of the last block

            let last_block = self.blocks.last().unwrap();

            if block.header.prev_block_hash != last_block.hash() {

                println!("prev hash is wrong");
                return Err(BtcError::InvalidBlock);
            }

            // check if the block's hash is less than the target

            if !block.header.hash().matches_target(block.header.target) {

                println!("does not match the target");
                return Err(BtcError::InvalidBlock);
            }

            // check if the block's merkle root is correct

            let calculated_merkle_root = MerkleRoot::calculate(&block.transactions);

            if calculated_merkle_root != block.header.merkle_root {


                println!("invalid merkle root");
                return Err(BtcError::InvalidMerkleRoot);
            }

            // check if the block's timestamp is after the last block's timestamp 
            if block.header.timestamp <= last_block.header.timestamp {

                return Err(BtcError::InvalidBlock);
            }

            // verify the all the transaction in the block

            block.verify_transactions(self.blocks_height(), &self.utxos)?;

            
            

        }

        // Remove the transaction from mempool that are now in the block

        let block_transactions: HashSet<_> = block.transactions.iter()
            .map(|tx| tx.hash())
            .collect();

        self.mempool.retain(|(_, tx)| {

            !block_transactions.contains(&tx.hash())

        });

        self.blocks.push(block);

        self.try_adjust_target();

        Ok(())
    }


    pub fn try_adjust_target(&mut self) {

        if self.blocks.is_empty() {

            return;
        }

        if self.blocks.len() % crate::DIFFICULTY_UPDATE_INTERVAL as usize != 0 {

            return;
        }

        // measure the time it took mine the last blocks

        let start_time = self.blocks[self.blocks.len() - crate::DIFFICULTY_UPDATE_INTERVAL as usize].header.timestamp;

        let end_time = self.blocks.last().unwrap().header.timestamp;

        let time_diff = end_time - start_time;

        // convert time_diff to seconds

        let time_diff_seconds = time_diff.num_seconds();

        // calculate the ideal number of seconds

        let target_seconds =  crate::IDEAL_BLOCK_TIME * crate::DIFFICULTY_UPDATE_INTERVAL;

        // let new_target = self.target * (time_diff_seconds as f64 / target_seconds as f64) as usize;

        let new_target = BigDecimal::parse_bytes(&self.target.to_string().as_bytes(), 10)
            .expect("bug")
                * (BigDecimal::from(time_diff_seconds)  
                    /  BigDecimal::from(target_seconds));

            // cut off decimal point and everything after it from the string representation

        let new_target_str = new_target.to_string().split('.').next().expect("bug expected a decimal point").to_owned();

        let new_target: U256 = U256::from_str_radix(&new_target_str, 10).expect("bug");


        // clamp new_target to within range of 4 * self.target and self.target / 4
       // we can multiply or divide either by 1, 2, 3, 4

        let new_target  = if new_target < self.target / 4 {

            self.target / 4

        } else if new_target > self.target * 4  {

            self.target * 4


        } else {

            new_target
        };


        // if the new_target is more than the minimum target 
        // set it to the minimm target

        self.target = new_target.min(crate::MIN_TARGET);



    }




    pub fn blocks_height(&self) -> u64 {
        
        self.blocks.len() as u64
    }
    

    // utxo's

    pub fn utxos(&self) -> &HashMap<Hash, (bool, TransactionOutput)> {

        &self.utxos
    }

    // target
    pub fn target(&self) -> U256 {

        self.target
    }

    // blocks

    pub fn blocks(&self) -> impl Iterator<Item = &Block> {

        self.blocks.iter()
    }


    // mempool

    pub fn mempool (&self) -> &[(DateTime<Utc>, Transaction)] {

        // later we also need to track of time

        &self.mempool
    }


    // now we need to teach blockchain to receive and add transaction to mempool

    pub fn add_to_mempool(&mut self, transactions: Transaction) -> Result<()> {


        // output should be less or equal to inputs
        // all inputs must have known UTXO
        // all inputs must be unique(no double spending)

    //         There is yet another security issue we must take care of: So far, it is still possible
    // to add multiple transactions to the mempool that reference the same unspent
    // transaction outputs. Furthermore, transactions in the mempool will stay stuck
    // there until the node is restarted (at which point, other nodes would share the same
    // transactions with the node again).
    // Therefore we need a mechanism to detect this type of double-spending, to discard
    // old unprocessed transactions, and to replace transactions with newer transactions
    // that reference the same inputs (which will prevent a potential double-spending
    // problem). To do this, we will need to make two adjustments:
    // ● Track the time when a particular transaction was inserted into the mem-
    // pool, and dump it if it has been there for too long.
    // ● Mark UTXOs that are being referenced by a transaction in mempool, and
    // find and remove the old transaction that marks those UTXOs.


        // validate transaction before insertion
        // all inputs must match known UTXO's and must be unique

        let mut known_inputs = HashSet::new();

        for input in &transactions.inputs {

            if !self.utxos.contains_key(&input.prev_transaction_output_hash) {

                return Err(BtcError::InvalidTransaction);
            }

            if known_inputs.contains(&input.prev_transaction_output_hash) {

                return Err(BtcError::InvalidTransaction);
            }

            known_inputs.insert(input.prev_transaction_output_hash);
        
        }

        // check if any of the utxos have the bool mark set to true 
        // and if so, find the transaction that reference them in mempool
        // remove it and set all utxo it refernce to false
        


        for input in &transactions.inputs {

            if let Some((true, _)) = self.utxos.get(&input.prev_transaction_output_hash) {

                // find the transaction that references th utxo

                let referencing_transaction: Option<(usize, &(DateTime<Utc>, Transaction))> = self.mempool.iter()
                    .enumerate()
                    .find(|( _, (_, transaction))| {

                        transaction.outputs
                        .iter()
                        .any(|output| {
                            output.hash() == input.prev_transaction_output_hash


                        })


                    } );

                    // if we found one, unmark all of its utxos

                if let Some((idx, (_, referencing_transaction))) = referencing_transaction {

                    for input in &referencing_transaction.inputs {

                        // set all the utxo's to false

                        self.utxos.entry(input.prev_transaction_output_hash).and_modify(|(marked, _)| {

                            *marked = false;


                        });


                    }

                    // remove the transaction from the mempool

                    self.mempool.remove(idx);

                } else {

                    // if somehow , there is no matching transaction, set this utxo to false

                    self.utxos.entry(input.prev_transaction_output_hash).and_modify(|(marked, _)| {

                        *marked = false;
                    });






                }


            }
        }

     

        // sort by miner fee


        let all_inputs = transactions.inputs
            .iter()
            .map(|input| {

                self.utxos.get(&input.prev_transaction_output_hash)
                    .expect("bug")
                    .1
                    .value

            })
            .sum::<u64>();

        let all_ouputs: u64 = transactions.outputs
            .iter()
            .map(|output| output.value)
            .sum();


        if all_inputs < all_ouputs {

            return Err(BtcError::InvalidTransaction);
        }


        // Mark the UTXO's as used
 
        for input in &transactions.inputs {

            self.utxos.entry(input.prev_transaction_output_hash).and_modify(|(marked, _)| {

                *marked = true;


            });
        }

        self.mempool.push((Utc::now(), transactions));

        // sort by miner fee

        self.mempool.sort_by_key(|(_, transaction)| {

            let all_inputs = transaction
                .inputs
                .iter()
                .map(|input| {
                    self.utxos
                        .get(&input.prev_transaction_output_hash)
                        .expect("bug")
                        .1
                        .value

                })
                .sum::<u64>();


            let all_outputs: u64 = transaction
                .outputs
                .iter()
                .map(|output| output.value)
                .sum();

            let miner_fee = all_inputs - all_outputs;

            miner_fee





        });

        Ok(())

       
    }



    // remove transaction older than Max_Mempool_transaction_age


    pub fn cleanup_mempool(&mut self) {


        let now = Utc::now();

        let mut utxo_hashes_to_unmark: Vec<Hash> = vec![];

        self.mempool.retain(|(timestamp, transaction )| {

            if now - timestamp > chrono::Duration::seconds(crate::MAX_MEMPOOL_TRANSACTION_AGE as i64) {

                // push all utxos to unmark to the vector
                // so we can unmark them later

                utxo_hashes_to_unmark.extend(transaction.inputs.iter().map(|input| {

                    input.prev_transaction_output_hash

             
                }) );

                false


            } else {

                true
            }

        });

            // unmark all the utxos

            for hash in utxo_hashes_to_unmark {

                self.utxos.entry(hash).and_modify(|(marked, _)| {

                    *marked = false
                });
            }



     


        }



    
       pub fn calculate_block_reward(&self) -> u64 {

          let block_height = self.blocks_height();

          let halving = block_height / crate::HALVING_INTERVAL;

          (crate::INITIAL_REWARD * 10u64) >> halving
       }





}
