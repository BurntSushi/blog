#![allow(dead_code, unused_imports, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    enum Option<T> {
      None,
      Some(T),
  }
  
  impl<T> Option<T> {
      fn unwrap(self) -> T {
          match self {
              Option::Some(val) => val,
              Option::None =>
                panic!("called `Option::unwrap()` on a `None` value"),
          }
      }
  }
    Ok(())
}

fn main() {
    main2().unwrap();
}
