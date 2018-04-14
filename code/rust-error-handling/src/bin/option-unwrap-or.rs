#![allow(dead_code, unused_imports, unused_macros, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    fn unwrap_or<T>(option: Option<T>, default: T) -> T {
      match option {
          None => default,
          Some(value) => value,
      }
  }
    Ok(())
}

fn main() {
    main2().unwrap();
}
