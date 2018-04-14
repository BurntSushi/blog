#![allow(dead_code, unused_imports, unused_macros, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    trait From<T> {
      fn from(T) -> Self;
  }
    Ok(())
}

fn main() {
    main2().unwrap();
}
