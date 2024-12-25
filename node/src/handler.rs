// 

use btc_lib::sha256::Hash;
use chrono::Utc;
use uuid::Uuid;
use tokio::net::TcpStream;
use btc_lib::network::Message;
use btc_lib::types::{
Block, BlockHeader, Transaction, TransactionOutput,
};
use btc_lib::util::MerkleRoot;


pub async fn handle_connection(mut socket: TcpStream) {

    loop {

        // read the message from socket
        let message = match Message::receive_async(&mut socket).await {


            Ok(message) => message,

            Err(e) => {

                println!("Invalid message from peer: {e}, closing that connection");

                return ;
            }
        };

        //         These are messages that the node sends as a response to either a miner or the wallet.
        // We should never receive them as the node, so we can just safely ignore them and
        // terminate the connection by returning from the function
       
        use btc_lib::network::Message::*;
        match message  {

            UTXOs(_) | Template(_) | Difference(_) | TemplateValidity(_) | NodeList(_) => {

                println!("I am neither a miner nor a wallet ! goodbye");

                return ;



            }

            FetchBlock(height) => {

                let blockchain = crate::BLOCKCHAIN.read().await;

                let Some(block) = blockchain.blocks().nth(height as usize).cloned()

                    else {

                        return ;
                    };

                let message = NewBlock(block);

                message.send_async(&mut socket).await.unwrap();
            }

            // DiscoverNodes { no filtering going on - we just send all nodes we know};
           

            DiscoverNodes =>  {

                let nodes = crate::NODES.iter()
                    .map(|x| x.key().clone())
                    .collect::<Vec<_>>();

                let message = NodeList(nodes);

                message.send_async(&mut socket).await.unwrap();
            }


            // AskDifference( read and subtract)

            AskDifference(height) => {

                let blockchain = crate::BLOCKCHAIN.read().await;

                let count = blockchain.blocks_height() as i32 - height as i32;

                let message = Difference(count);

                message.send_async(&mut socket).await.unwrap();
            }


            //Returning UTXOs for a particular public key is a bit more involved process, as we
            // need to filter them out, and separate them from the tags marking them

           FetchUTXOs(key) => {

                println!("received request to fetch Utxos");

                let blockchain = crate::BLOCKCHAIN.read().await;

                let utxos = blockchain.utxos().iter()
                    
                    .filter(|( _,  ( _, txout))| {
                       
                        txout.pubkey == key

                    })
                    .map(|(_, (marked, txout))| {

                        (txout.clone(), *marked)


                    })
                    .collect::<Vec<_>>();

                let message = UTXOs(utxos);

                message.send_async(&mut socket).await.unwrap();

            
            }


            NewBlock(block) => {

                let mut blockchain = crate::BLOCKCHAIN.write().await;

                println!("received new block");

                if blockchain.add_block(block).is_err() {

                    println!("block rejected");
                }
            }


            //  We are making a simplification here in that we just add it to the mempool. It would
            // be a nice idea to send it back to other nodes that may not have it. However, we would
            // have to add a mechanism for preventing the network from creating notification
            // loops. You can try implementing one, if you wan

            NewTransaction(tx) => {

                let mut blockchain = crate::BLOCKCHAIN.write().await;

                println!("recieved transactionfrom friend");

                if blockchain.add_to_mempool(tx).is_err() {

                    println!("transaction rejected, closing connection");

                    return
                }
            }


            ValidateTemplate(block_template) => {

                let blockchain = crate::BLOCKCHAIN.read().await;

                let status = block_template.header.prev_block_hash == blockchain.blocks()
                    .last()
                    .map(|last_block| last_block.hash())
                    .unwrap_or(Hash::zero());

                let message = TemplateValidity(status);

                message.send_async(&mut socket).await.unwrap();
            }


            // if miner send us a correctly mined block, we want to broadcast it to other nodes

            SubmitTemplate(block) => {

                println!("received allegedly mined template");

                let mut blockchain = crate::BLOCKCHAIN.write().await;

                if let Err(e) = blockchain.add_block(block.clone()) {

                    println!("block rejected {e} closing connection");

                    return;


                }

                blockchain.rebuild_utxos();

                println!("blocks looks good, broadcasting");

                // send all blocks to all friend nodes

                let nodes = crate::NODES.iter()
                    .map(|x| x.key().clone())
                    .collect::<Vec<_>>();

                for node in nodes {

                    if let Some(mut stream) = crate::NODES.get_mut(&node) {


                        let message = Message::NewBlock(block.clone());

                        if message.send_async(&mut *stream).await.is_err() {

                            println!("failed to send block to {}", node);
                        }
                    }
                }
            }



            SubmitTransaction(tx) => {

                println!("submit tx");

                let mut blockchain = crate::BLOCKCHAIN.write().await;

                if let Err(e) = blockchain.add_to_mempool(tx.clone()) {

                    println!("transaction rejected, closing connection: {e}");

                    return ;

                }

                println!("added transaction to mempool");

                // send  transaction to all friend nodes

                let nodes = crate::NODES.iter()
                    .map(|x| x.key().clone())
                    .collect::<Vec<_>>();

                for node in nodes {

                    println!("sending to friend : {node }");

                    if let Some(mut stream) = crate::NODES.get_mut(&node) {
                    
                        let message = Message::NewTransaction(tx.clone());

                        if message.send_async(&mut *stream).await.is_err() {

                            println!("failed to send transaction to {}", node);
                        
                        
                        }
                    
                    }
                }
            }



            // fetching template

            FetchTemplate(pubkey) => {

                let blockchain = crate::BLOCKCHAIN.read().await;

                let mut transactions = vec![];

                // insert transaction from mempool

                transactions.extend(
                    
                    blockchain
                        .mempool()
                        .iter()
                        .take(btc_lib::BLOCK_TRANSACTION_CAP)
                        .map(|(_, tx)| tx)
                        .clone()
                        .collect::<Vec<_>>(),
                
                
                );

                // insert coinbase tx with pubkey
                let t = Transaction {

                                inputs: vec![],

                                outputs: vec![ TransactionOutput {
                                    
                                    pubkey,
                                    
                                    unique_id: Uuid::new_v4(),

                                    value: 0,
                                
                                
                                }],
                            };
                transactions.insert(0, &t);


                let merkle_root = MerkleRoot::calculate(&transactions.iter().map(|&t| t.clone()).collect::<Vec<_>>());

                let mut block  = Block::new(
                    
                    BlockHeader {

                                timestamp: Utc::now(),

                                prev_block_hash: blockchain.blocks()
                                    .last()
                                    .map(| last_block|  {
                                    
                                        last_block.hash()
                                        
                                })
                                .unwrap_or(Hash::zero()),

                                nonce: 0,

                                target: blockchain.target(),

                                merkle_root,
                                
                            },

                       transactions.into_iter().cloned().collect(),


                );

                let miner_fees = match block.calculate_miner_fees(blockchain.utxos()) {
                
                    Ok(fees) => fees,

                    Err(e) => {
                    
                        eprintln!("{e}");

                        return;

                    }
                
                };

                let reward = blockchain.calculate_block_reward();

                // update coinbase tx with reward

                block.transactions[0].outputs[0].value = reward + miner_fees;


                // recalculate merkle root

                block.header.merkle_root = MerkleRoot::calculate(&block.transactions);

                let message = Template(block);

                message.send_async(&mut socket).await.unwrap();

                


            
            
            
            }

            






        }


    }
}