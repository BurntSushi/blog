#![allow(dead_code, unused_imports, unused_macros, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    // We've seen all these imports before except for Levenshtein.
  // Levenshtein is a type that knows how to build Levenshtein automata.
  use fst::{IntoStreamer, Streamer, Levenshtein, Set};
  
  let keys = vec!["fa", "fo", "fob", "focus", "foo", "food", "foul"];
  let set = Set::from_iter(keys)?;
  
  // Build our fuzzy query. This says to search for "foo" and return any keys
  // that have a Levenshtein distance from "foo" of no more than 1.
  let lev = Levenshtein::new("foo", 1)?;
  
  // Apply our fuzzy query to the set we built and turn the query into a stream.
  let stream = set.search(lev).into_stream();
  
  // Get the results and confirm that they are what we expect.
  let keys = stream.into_strs()?;
  assert_eq!(keys, vec![
      "fo",   // 1 deletion
      "fob",  // 1 substitution
      "foo",  // 0 insertions/deletions/substitutions
      "food", // 1 insertion
  ]);
    Ok(())
}

fn main() {
    main2().unwrap();
}
