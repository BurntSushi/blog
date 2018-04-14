#![allow(dead_code, unused_imports, unused_macros, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    // Imports the `File` type into this scope and the entire `std::io` module.
  use std::fs::File;
  use std::io;
  
  // Imports the `SetBuilder` type from the `fst` module.
  use fst::SetBuilder;
  
  // Create a file handle that will write to "set.fst" in the current directory.
  let file_handle = File::create("set.fst")?;
  
  // Make sure writes to the file are buffered.
  let buffered_writer = io::BufWriter::new(file_handle);
  
  // Create a set builder that streams the data structure to set.fst.
  // We could use a socket here, or an in memory buffer, or anything that
  // is "writable" in Rust.
  let mut set_builder = SetBuilder::new(buffered_writer)?;
  
  // Insert a few keys from the greatest band of all time.
  // An insert can fail in one of two ways: either a key was inserted out of
  // order or there was a problem writing to the underlying file.
  set_builder.insert("bruce")?;
  set_builder.insert("clarence")?;
  set_builder.insert("stevie")?;
  
  // Finish building the set and make sure the entire data structure is flushed
  // to disk. After this is called, no more inserts are allowed. (And indeed,
  // are prevented by Rust's type/ownership system!)
  set_builder.finish()?;
    Ok(())
}

fn main() {
    main2().unwrap();
}
