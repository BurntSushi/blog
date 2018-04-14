#![allow(dead_code, unused_imports, unused_macros, unused_variables)]
extern crate fst;
extern crate fst_levenshtein;
extern crate fst_regex;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    use std::str::from_utf8;
  
  use fst::{Streamer, Set};
  use fst::set;
  use fst_regex::Regex;
  
  // Create 5 sets. As a convenience, these are stored in memory, but they could
  // just as easily have been memory mapped from disk using `Set::from_path`.
  let set1 = Set::from_iter(&["AC/DC", "Aerosmith"])?;
  let set2 = Set::from_iter(&["Bob Seger", "Bruce Springsteen"])?;
  let set3 = Set::from_iter(&["George Thorogood", "Golden Earring"])?;
  let set4 = Set::from_iter(&["Kansas"])?;
  let set5 = Set::from_iter(&["Metallica"])?;
  
  // Build our regular expression query.
  let spaces = Regex::new(r".*\s.*")?;
  
  // Build a set operation. All we need to do is add a stream from each set and
  // ask for the union. (Other operations, such as `intersection`, are also
  // available.)
  let mut stream =
      set::OpBuilder::new()
      .add(set1.search(&spaces))
      .add(set2.search(&spaces))
      .add(set3.search(&spaces))
      .add(set4.search(&spaces))
      .add(set5.search(&spaces))
      .union();
  
  // This is the same as the previous example, except our search narrowed our
  // results down a bit.
  let mut keys = vec![];
  while let Some(key) = stream.next() {
      let key = from_utf8(key)?.to_string();
      keys.push(key);
  }
  assert_eq!(keys, vec![
      "Bob Seger", "Bruce Springsteen", "George Thorogood", "Golden Earring",
  ]);
    Ok(())
}

fn main() {
    main2().unwrap();
}
