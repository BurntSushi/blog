#![allow(dead_code, unused_imports, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    use std::fs::File;
  use std::io::Read;
  
  use fst::Set;
  
  // Open a handle to a file and read its entire contents into memory.
  let mut file_handle = try!(File::open("set.fst"));
  let mut bytes = vec![];
  try!(file_handle.read_to_end(&mut bytes));
  
  // Construct the set.
  let set = try!(Set::from_bytes(bytes));
  
  // Finally, we can query.
  println!("number of elements: {}", set.len());
    Ok(())
}

fn main() {
    main2().unwrap();
}
