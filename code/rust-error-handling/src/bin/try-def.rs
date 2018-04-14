#![allow(dead_code, unused_imports, unused_macros, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    macro_rules! try {
      ($e:expr) => (match $e {
          Ok(val) => val,
          Err(err) => return Err(::std::convert::From::from(err)),
      });
  }
    Ok(())
}

fn main() {
    main2().unwrap();
}
