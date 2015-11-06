#![allow(dead_code, unused_imports, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    fn ok_or<T, E>(option: Option<T>, err: E) -> Result<T, E> {
      match option {
          Some(val) => Ok(val),
          None => Err(err),
      }
  }
    Ok(())
}

fn main() {
    main2().unwrap();
}
