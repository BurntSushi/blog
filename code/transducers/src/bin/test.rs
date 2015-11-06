#![allow(dead_code, unused_imports, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    use fst::{IntoStreamer, Streamer, Levenshtein, Set};
  
  // A convenient way to create sets in memory.
  let keys = vec!["fa", "fo", "fob", "focus", "foo", "food", "foul"];
  let set = try!(Set::from_iter(keys));
  
  // Build our fuzzy query.
  let lev = try!(Levenshtein::new("foo", 1));
  
  // Apply our fuzzy query to the set we built.
  let stream = set.search(lev).into_stream();
  let keys = try!(stream.into_strs());
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
