#![allow(dead_code, unused_imports, unused_macros, unused_variables)]
extern crate fst;
extern crate fst_levenshtein;
extern crate fst_regex;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    use std::fs::File;
  use std::io::Read;
  
  use fst::Set;
  
  // Open a handle to a file and read its entire contents into memory.
  let mut file_handle = File::open("set.fst")?;
  let mut bytes = vec![];
  file_handle.read_to_end(&mut bytes)?;
  
  // Construct the set.
  let set = Set::from_bytes(bytes)?;
  
  // Finally, we can query.
  println!("number of elements: {}", set.len());
    Ok(())
}

fn main() {
    main2().unwrap();
}
