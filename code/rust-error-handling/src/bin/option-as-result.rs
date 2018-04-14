#![allow(dead_code, unused_imports, unused_macros, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    type Option<T> = Result<T, ()>;
    Ok(())
}

fn main() {
    main2().unwrap();
}
