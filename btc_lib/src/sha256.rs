use crate::U256;
use std::fmt;

use sha256::digest;
use serde::{ Deserialize, Serialize };

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub struct Hash(U256);

impl fmt::Display for Hash {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        write!(f, "{:x}", self.0)
    }


}




impl Hash {

    // ...
    // convert to bytes
    pub fn as_bytes(&self) -> [u8; 32]  {

        //let mut bytes: Vec<u8> = vec![0; 32];

        self.0.to_little_endian().as_slice().try_into().unwrap()
    }
    }

impl Hash {

    // hash anything that can be serde Serialized via ciborium

    pub fn hash<T: serde::Serialize>(data: &T) -> Self {

        let mut serialized: Vec<u8> = vec![];

        if let Err(e) = ciborium::into_writer(
            data,
            &mut serialized
        ) {

            panic!("failed to serialize the data {:?}", e);
        }

        let hash = digest(&serialized);
        let hash_bytes = hex::decode(hash).unwrap();
        let hash_array: [u8; 32] = hash_bytes.as_slice().try_into().unwrap();

        // @note check this , it can cause the problem. big_endian method, which interprets the array directly
        let x = U256::from_big_endian(&hash_array);



        Hash(x)

       


    }


    // check if a hash matches, a target

    pub fn matches_target(&self, target: U256) -> bool {

        self.0 <= target
    }

    pub fn zero() -> Self {

        Hash(U256::zero())
    }
}