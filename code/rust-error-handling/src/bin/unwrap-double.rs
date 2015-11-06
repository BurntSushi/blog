#![allow(dead_code, unused_imports, unused_variables)]
extern crate fst;

use std::env;

fn main() {
    let mut argv = env::args();
    let arg: String = argv.nth(1).unwrap(); // error 1
    let n: i32 = arg.parse().unwrap(); // error 2
    println!("{}", 2 * n);
}

// $ cargo run --bin unwrap-double 5
// 10
