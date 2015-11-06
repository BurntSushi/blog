#![allow(dead_code, unused_imports, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    fn and_then<F, T, A>(option: Option<T>, f: F) -> Option<A>
          where F: FnOnce(T) -> Option<A> {
      match option {
          None => None,
          Some(value) => f(value),
      }
  }
    Ok(())
}

fn main() {
    main2().unwrap();
}
