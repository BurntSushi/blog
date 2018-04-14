#![allow(dead_code, unused_imports, unused_macros, unused_variables)]
extern crate fst;
extern crate fst_levenshtein;
extern crate fst_regex;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    use std::str::from_utf8; // converts UTF-8 bytes to a Rust string
  
  // We import the usual `Set`, but also include `Streamer`, which is a trait
  // that makes it possible to call `next` on a stream.
  use fst::{Streamer, Set};
  
  // Store the keys somewhere so that we can compare what we get with them and
  // make sure they're the same.
  let keys = vec!["bruce", "clarence", "danny", "garry", "max", "roy", "stevie"];
  
  // Pass a reference with `&keys`. If we had just used `keys` instead, then it
  // would have *moved* into `Set::from_iter`, which would prevent us from using
  // it below to check that the keys we got are the same as the keys we gave.
  let set = Set::from_iter(&keys)?;
  
  // Ask the set for a stream of all of its keys.
  let mut stream = set.stream();
  
  // Iterate over the elements and collect them.
  let mut got_keys = vec![];
  while let Some(key) = stream.next() {
      // Keys are byte sequences, but the keys we inserted are strings.
      // Strings in Rust are UTF-8 encoded, so we need to decode here.
      let key = from_utf8(key)?.to_string();
      got_keys.push(key);
  }
  assert_eq!(keys, got_keys);
    Ok(())
}

fn main() {
    main2().unwrap();
}
