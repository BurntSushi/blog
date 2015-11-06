#![allow(dead_code, unused_imports, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    use std::error::Error;
  use std::fs::File;
  use std::io::Read;
  use std::path::Path;
  
  fn file_double<P: AsRef<Path>>(file_path: P) -> Result<i32, Box<Error>> {
      let mut file = try!(File::open(file_path));
      let mut contents = String::new();
      try!(file.read_to_string(&mut contents));
      let n = try!(contents.trim().parse::<i32>());
      Ok(2 * n)
  }
    Ok(())
}

fn main() {
    main2().unwrap();
}
