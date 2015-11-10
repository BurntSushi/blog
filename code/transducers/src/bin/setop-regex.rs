#![allow(dead_code, unused_imports, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    use std::str::from_utf8;
  
  use fst::{Streamer, Regex, Set};
  use fst::set;
  
  // Create 5 sets. As a convenience, these are stored in memory, but they could
  // just as easily have been memory mapped from disk using `Set::from_path`.
  let set1 = try!(Set::from_iter(&["AC/DC", "Aerosmith"]));
  let set2 = try!(Set::from_iter(&["Bob Seger", "Bruce Springsteen"]));
  let set3 = try!(Set::from_iter(&["George Thorogood", "Golden Earring"]));
  let set4 = try!(Set::from_iter(&["Kansas"]));
  let set5 = try!(Set::from_iter(&["Metallica"]));
  
  // Build our regular expression query.
  let spaces = try!(Regex::new(r".*\s.*"));
  
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
      let key = try!(from_utf8(key)).to_owned();
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
