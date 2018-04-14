#![allow(dead_code, unused_imports, unused_macros, unused_variables)]
extern crate fst;
extern crate fst_levenshtein;
extern crate fst_regex;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    use fst::Set;
  
  // Construct the set from a file path. The fst crate implements this using a
  // memory map, which is why this method is unsafe to call. Callers must ensure
  // that they do not open another mutable memory map in the same program.
  let set = unsafe { Set::from_path("set.fst")? };
  
  // Finally, we can query. This can happen immediately, without having
  // to read the entire set into memory.
  println!("number of elements: {}", set.len());
    Ok(())
}

fn main() {
    main2().unwrap();
}
