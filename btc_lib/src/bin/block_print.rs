use btc_lib::types::Block;
use btc_lib::util::Saveable;
use std::env;
use std::process::exit;
use std::fs::File;





fn main() {

    let path = if let Some(arg) = env::args().nth(1) {

        arg

    } else {

        eprintln!("usage : block_print < block_file >");

        exit(1);
    };

    if let Ok(file) = File::open(path) {

        let block = Block::load(file).expect("failed to load the block");

        println!("{:#?}", block);
    }

}