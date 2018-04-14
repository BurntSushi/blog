#![allow(dead_code, unused_imports, unused_macros, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    use fst::Map;
  
  let map = Map::from_iter(vec![
    ("bruce", 1972),
    ("clarence", 1972),
    ("stevie", 1975),
  ])?;
    Ok(())
}

fn main() {
    main2().unwrap();
}
