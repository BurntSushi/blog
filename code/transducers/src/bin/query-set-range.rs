#![allow(dead_code, unused_imports, unused_macros, unused_variables)]
extern crate fst;
extern crate fst_levenshtein;
extern crate fst_regex;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    // We now need the IntoStreamer trait, which provides a way to convert a
  // range query into a stream.
  use fst::{IntoStreamer, Streamer, Set};
  
  // Same as previous example.
  let keys = vec!["bruce", "clarence", "danny", "garry", "max", "roy", "stevie"];
  let set = Set::from_iter(&keys)?;
  
  // Build a range query that includes all keys greater than or equal to `c`
  // and less than or equal to `roy`.
  let range = set.range().ge("c").le("roy");
  
  // Turn the range into a stream.
  let stream = range.into_stream();
  
  // Use a convenience method defined on streams to collect the elements in the
  // stream into a sequence of strings. This is effectively a shorter form of the
  // `while let` loop we wrote out in the previous example.
  let got_keys = stream.into_strs()?;
  
  // Check that we got the right keys.
  assert_eq!(got_keys, &keys[1..6]);
    Ok(())
}

fn main() {
    main2().unwrap();
}
