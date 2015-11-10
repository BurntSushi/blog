#![allow(dead_code, unused_imports, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    use fst::raw::{Builder, Fst, Output};
  
  // The Fst type has a separate builder just like sets and maps.
  let mut builder = Builder::memory();
  builder.insert("bar", 1).unwrap();
  builder.insert("baz", 2).unwrap();
  builder.insert("foo", 3).unwrap();
  
  // Finish construction and get the raw bytes of the fst.
  let fst_bytes = try!(builder.into_inner());
  
  // Create an Fst that we can query.
  let fst = try!(Fst::from_bytes(fst_bytes));
  
  // Basic querying.
  assert!(fst.contains_key("foo"));
  assert_eq!(fst.get("abc"), None);
  
  // Looking up a value returns an `Output` instead of a `u64`.
  // This is the internal representation of an output on a transition.
  // The underlying u64 can be accessed with the `value` method.
  assert_eq!(fst.get("baz"), Some(Output::new(2)));
  
  // Methods like `stream`, `range` and `search` are also available, which
  // function the same way as they do for sets and maps.
    Ok(())
}

fn main() {
    main2().unwrap();
}
