use btc_lib::types::Transaction;
use btc_lib::util::Saveable;
use std::env;
use std::fs::File;
use std::process::exit;



fn main() {

   

    let path = if let Some(arg) = env::args().nth(1) {   // first one is executable

        arg

    } else {

        eprintln!("usage: tx_print <tx_file> ");

        exit(1);


    };


    if let Ok(file) = File::open(path) {

        let tx = Transaction::load(file) .expect("Failed to load transaction");
       
        println!("{:#?}", tx);

        }




}