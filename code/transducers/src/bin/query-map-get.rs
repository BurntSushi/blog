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
  
  // Maps have `contains_key`, which is just like a set's `contains`:
  assert!(map.contains_key("bruce"));    // "bruce" is in the map
  assert!(!map.contains_key("andrew"));  // "andrew" is not
  
  // Maps also have `get`, which retrieves the value if it exists.
  // `get` returns an `Option<u64>`, which is something that can either be
  // empty (when the key does not exist) or present with the value.
  assert_eq!(map.get("bruce"), Some(1972)); // bruce joined the band in 1972
  assert_eq!(map.get("andrew"), None);      // andrew was never in the band
    Ok(())
}

fn main() {
    main2().unwrap();
}
