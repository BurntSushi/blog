#![allow(dead_code, unused_imports, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    use std::num::ParseIntError;
  use std::result;
  
  type Result<T> = result::Result<T, ParseIntError>;
  
  fn double_number(number_str: &str) -> Result<i32> {
      unimplemented!();
  }
    Ok(())
}

fn main() {
    main2().unwrap();
}
