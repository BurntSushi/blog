#![allow(dead_code, unused_imports, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    // We now need the IntoStreamer trait, which provides a way to convert a
  // range query into a stream.
  use fst::{IntoStreamer, Streamer, Set};
  
  // Same as previous example.
  let keys = vec!["bruce", "clarence", "danny", "garry", "max", "roy", "stevie"];
  let set = try!(Set::from_iter(&keys));
  
  // Build a range query that includes all keys greater than or equal to `c`
  // and less than or equal to `roy`.
  let range = set.range().ge("c").le("roy");
  
  // Turn the range into a stream.
  let stream = range.into_stream();
  
  // Use a convenience method defined on streams to collect the elements in the
  // stream into a sequence of strings. This is effectively a shorter form of the
  // `while let` loop we wrote out in the previous example.
  let got_keys = try!(stream.into_strs());
  
  // Check that we got the right keys.
  assert_eq!(keys[1..6].to_vec(), got_keys);
    Ok(())
}

fn main() {
    main2().unwrap();
}
