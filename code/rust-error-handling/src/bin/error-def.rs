#![allow(dead_code, unused_imports, unused_macros, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    use std::fmt::{Debug, Display};
  
  trait Error: Debug + Display {
    /// A short description of the error.
    fn description(&self) -> &str;
  
    /// The lower level cause of this error, if any.
    fn cause(&self) -> Option<&Error> { None }
  }
    Ok(())
}

fn main() {
    main2().unwrap();
}
