
use crate::sha256::Hash;
use crate::types::Transaction;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write, Result as IoResult};
use std::fs::File;
use std::path::Path;


#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MerkleRoot(Hash);

impl MerkleRoot {

    // calculate the merkle root of the of block's transaction 


    pub fn calculate(
        transactions: &[Transaction],

    ) -> MerkleRoot {

        let mut layer: Vec<Hash> = vec![];

        for transaction in transactions {

            layer.push(Hash::hash(transaction));
        }
        

        while layer.len() > 1 {

            let mut new_layer = vec![];

            for pair in layer.chunks(2) {

                let left = pair[0];

                // if there is no right , use the left hash again

                let right = pair.get(1).unwrap_or((&pair[0]));

                new_layer.push(Hash::hash(&[left, *right]));
            }

            layer = new_layer;
        }

        MerkleRoot(layer[0])

    }
}



// We added the Self: Sized where clause. This trait bound is required because not
// all types have a size known at compile time (for example, &str has a known size,
// it is the size of a fat pointer40, but the underlying str does not have a size known
// at compile-time, since it refers to the real bytes of the string)
pub trait Saveable 
    where Self: Sized,               // the Sized trait in Rust refers to types whose size is known at compile time
{

    fn load<I: Read>(reader: I) -> IoResult<Self>;

    fn save<O: Write>(&self, writer: O) -> IoResult<()>;


//     The two new methods take P: AsRef<Path> as a generic parameter for the
// path variable instead of taking the Path type directly, and that tells us that we
// can use anything convertible into a path, which includes a &str string slice.

    fn save_to_file<P: AsRef<Path>>(&self, path: P,) -> IoResult<()> {

        let file = File::create(&path)?;

        self.save(file)
    }


    fn load_from_file<P: AsRef<Path>>(path: P,) -> IoResult<Self> {  


        //This creates a new file at the given path. 
        //If the file cannot be created (e.g., due to permissions or non-existent directories), 
        //the function returns an error (? is the "try" operator which propagates errors).

        let file = File::open(&path)?;

        Self::load(file)
    }
}