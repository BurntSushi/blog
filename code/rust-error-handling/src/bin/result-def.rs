#![allow(dead_code, unused_imports, unused_macros, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    enum Result<T, E> {
      Ok(T),
      Err(E),
  }
  
  impl<T, E: ::std::fmt::Debug> Result<T, E> {
      fn unwrap(self) -> T {
          match self {
              Result::Ok(val) => val,
              Result::Err(err) =>
                panic!("called `Result::unwrap()` on an `Err` value: {:?}", err),
          }
      }
  }
    Ok(())
}

fn main() {
    main2().unwrap();
}
