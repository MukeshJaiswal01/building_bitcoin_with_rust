use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::crypto::{PublicKey, Signature};
use crate::sha256::Hash;

use crate::util::Saveable;

use std::io:: {

    Error as IoError, ErrorKind as IoErrorKind, Read, Result as IoResult, Write,

};






#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transaction {

    pub inputs: Vec<TransactionInput>,
    pub outputs: Vec<TransactionOutput>,
}


// save and load expecting CBOR from ciborium as format

impl Saveable for Transaction {


    fn load<I: Read>(reader: I) -> IoResult<Self> {

        ciborium::de::from_reader(reader).map_err(|_| {

            IoError::new(IoErrorKind::InvalidData, "failed to deserialize data")
        })
    } 


    fn save<O: Write>(&self, writer: O) -> IoResult<()> {

        ciborium::ser::into_writer(self, writer).map_err(|_| {

            IoError::new(IoErrorKind::InvalidData, "failed to serialize transaction")
        })
    }



}





impl Transaction {

    pub fn new(
        inputs: Vec<TransactionInput>,
        outputs: Vec<TransactionOutput>
    ) -> Self {

        Transaction {
            inputs: inputs,
            outputs: outputs,
        }
    }


    pub fn hash(&self) -> Hash {

        Hash::hash(self)
         
    }
}




#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionInput {

// the hash of the transaction output, which
// we are linking into this transaction as input. Real bitcoin uses a slightly dif-
// ferent scheme - it stores the previous transaction hash, and the index of the
// output in that transaction.

    pub prev_transaction_output_hash: Hash,
    pub signature: Signature,

}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionOutput{

    pub value: u64,

// The unique_id is a generated identifier that helps us ensure that the hash of each
// transaction output is unique, and can be used to identify it
    pub unique_id: Uuid,
    pub pubkey: PublicKey,

}


impl TransactionOutput {

    pub fn hash(&self) -> Hash {

        Hash::hash(self)
    }
}
