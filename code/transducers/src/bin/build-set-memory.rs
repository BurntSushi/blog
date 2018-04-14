#![allow(dead_code, unused_imports, unused_macros, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    use fst::{Set, SetBuilder};
  
  // Create a set builder that streams the data structure to memory.
  let mut set_builder = SetBuilder::memory();
  
  // Inserts are the same as before.
  // They can still fail if they are inserted out of order, but writes to the
  // heap are (mostly) guaranteed to succeed. Since we know we're inserting these
  // keys in the right order, we use "unwrap," which will panic or abort the
  // current thread of execution if it fails.
  set_builder.insert("bruce").unwrap();
  set_builder.insert("clarence").unwrap();
  set_builder.insert("stevie").unwrap();
  
  // Finish building the set and get back a region of memory that can be
  // read as an FST.
  let fst_bytes = set_builder.into_inner()?;
  
  // And create a new Set with those bytes.
  // We'll cover this more in the next section on querying.
  let set = Set::from_bytes(fst_bytes).unwrap();
    Ok(())
}

fn main() {
    main2().unwrap();
}
