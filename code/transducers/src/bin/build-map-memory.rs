#![allow(dead_code, unused_imports, unused_macros, unused_variables)]
extern crate fst;
extern crate fst_levenshtein;
extern crate fst_regex;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    use fst::{Map, MapBuilder};
  
  // Create a map builder that streams the data structure to memory.
  let mut map_builder = MapBuilder::memory();
  
  // Inserts are the same as before, except we include a value with each key.
  map_builder.insert("bruce", 1972).unwrap();
  map_builder.insert("clarence", 1972).unwrap();
  map_builder.insert("stevie", 1975).unwrap();
  
  // These steps are exactly the same as before.
  let fst_bytes = map_builder.into_inner()?;
  let map = Map::from_bytes(fst_bytes).unwrap();
    Ok(())
}

fn main() {
    main2().unwrap();
}
