#![allow(dead_code, unused_imports, unused_macros, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    use fst::Set;
  
  let set = Set::from_iter(vec!["bruce", "clarence", "stevie"])?;
    Ok(())
}

fn main() {
    main2().unwrap();
}
