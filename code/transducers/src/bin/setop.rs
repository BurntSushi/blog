#![allow(dead_code, unused_imports, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    use std::str::from_utf8;
  
  use fst::{Streamer, Set};
  use fst::set;
  
  // Create 5 sets. As a convenience, these are stored in memory, but they could
  // just as easily have been memory mapped from disk using `Set::from_path`.
  let set1 = try!(Set::from_iter(&["AC/DC", "Aerosmith"]));
  let set2 = try!(Set::from_iter(&["Bob Seger", "Bruce Springsteen"]));
  let set3 = try!(Set::from_iter(&["George Thorogood", "Golden Earring"]));
  let set4 = try!(Set::from_iter(&["Kansas"]));
  let set5 = try!(Set::from_iter(&["Metallica"]));
  
  // Build a set operation. All we need to do is add a stream from each set and
  // ask for the union. (Other operations, such as `intersection`, are also
  // available.)
  let mut stream =
      set::OpBuilder::new()
      .add(&set1)
      .add(&set2)
      .add(&set3)
      .add(&set4)
      .add(&set5)
      .union();
  
  // Now collect all of the keys. `stream` is just like any other stream that
  // we've seen before.
  let mut keys = vec![];
  while let Some(key) = stream.next() {
      let key = try!(from_utf8(key)).to_owned();
      keys.push(key);
  }
  assert_eq!(keys, vec![
      "AC/DC", "Aerosmith", "Bob Seger", "Bruce Springsteen",
      "George Thorogood", "Golden Earring", "Kansas", "Metallica",
  ]);
    Ok(())
}

fn main() {
    main2().unwrap();
}
