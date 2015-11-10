#![allow(dead_code, unused_imports, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    // We've seen all these imports before except for Regex.
  // Regex is a type that knows how to build regular expression automata.
  use fst::{IntoStreamer, Streamer, Regex, Set};
  
  let keys = vec!["123", "food", "xyz123", "τροφή", "еда", "מזון", "☃☃☃"];
  let set = try!(Set::from_iter(keys));
  
  // Build a regular expression. This can fail if the syntax is incorrect or
  // if the automaton becomes too big.
  // This particular regular expression matches keys that are not empty and
  // only contain letters. Use of `\pL` here stands for "any Unicode codepoint
  // that is considered a letter."
  let lev = try!(Regex::new(r"\pL+"));
  
  // Apply our regular expression query to the set we built and turn the query
  // into a stream.
  let stream = set.search(lev).into_stream();
  
  // Get the results and confirm that they are what we expect.
  let keys = try!(stream.into_strs());
  
  // Notice that "123", "xyz123" and "☃☃☃" did not match.
  assert_eq!(keys, vec![
      "food",
      "τροφή",
      "еда",
      "מזון",
  ]);
    Ok(())
}

fn main() {
    main2().unwrap();
}
