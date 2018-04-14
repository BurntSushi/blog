#![allow(dead_code, unused_imports, unused_macros, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    use std::fs::File;
  use std::io::{self, Read};
  use std::num;
  use std::path::Path;
  
  // We derive `Debug` because all types should probably derive `Debug`.
  // This gives us a reasonable human readable description of `CliError` values.
  #[derive(Debug)]
  enum CliError {
      Io(io::Error),
      Parse(num::ParseIntError),
  }
  
  fn file_double_verbose<P: AsRef<Path>>(file_path: P) -> Result<i32, CliError> {
      let mut file = File::open(file_path).map_err(CliError::Io)?;
      let mut contents = String::new();
      file.read_to_string(&mut contents).map_err(CliError::Io)?;
      let n: i32 = contents.trim().parse().map_err(CliError::Parse)?;
      Ok(2 * n)
  }
  
  impl From<io::Error> for CliError {
      fn from(err: io::Error) -> CliError {
          CliError::Io(err)
      }
  }
  
  impl From<num::ParseIntError> for CliError {
      fn from(err: num::ParseIntError) -> CliError {
          CliError::Parse(err)
      }
  }
  
  fn file_double<P: AsRef<Path>>(file_path: P) -> Result<i32, CliError> {
      let mut file = File::open(file_path)?;
      let mut contents = String::new();
      file.read_to_string(&mut contents)?;
      let n: i32 = contents.trim().parse()?;
      Ok(2 * n)
  }
    Ok(())
}

fn main() {
    main2().unwrap();
}
