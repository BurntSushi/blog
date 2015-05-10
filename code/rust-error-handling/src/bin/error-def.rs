#![allow(dead_code, unused_imports, unused_variables)]
fn main() {
  use std::fmt::{Debug, Display};
  
  trait Error: Debug + Display {
    /// A short description of the error.
    fn description(&self) -> &str;
  
    /// The lower level cause of this error, if any.
    fn cause(&self) -> Option<&Error> { None }
  }
}
