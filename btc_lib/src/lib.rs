use uint::construct_uint;
use serde::{Deserialize, Serialize};
construct_uint! {

    //construct a unsigned 256 bit integer
    // consisting of 4 x 64 bit words
    
    #[derive(Serialize, Deserialize)]
    pub struct U256(4);
}



// initial reward in bitcoin - multiply by 10^8 to get satoshis
pub const INITIAL_REWARD: u64 = 50;

// halving interval in blocks
pub const HALVING_INTERVAL: u64 = 210;

// ideal block time in seconds
pub const IDEAL_BLOCK_TIME: u64 = 10;

// minimum target : MIN_TARGET number is encoded in such a weird
// way - it is little-endian. The least significant 64 bits are the last
pub const MIN_TARGET: U256 = U256([
0xFFFF_FFFF_FFFF_FFFF,
0xFFFF_FFFF_FFFF_FFFF,
0xFFFF_FFFF_FFFF_FFFF,
0x0000_FFFF_FFFF_FFFF,
]);

// difficulty update interval in blocks
pub const DIFFICULTY_UPDATE_INTERVAL: u64 = 50;

// maximum mempool transaction age in seconds
pub const MAX_MEMPOOL_TRANSACTION_AGE: u64 = 14 * 24 * 3600;

// maximum amount of transaction allowed in a block 

pub const BLOCK_TRANSACTION_CAP: usize = 20;





pub mod sha256;
pub mod types;
pub mod util;
pub mod crypto;
pub mod error;
pub mod network;

