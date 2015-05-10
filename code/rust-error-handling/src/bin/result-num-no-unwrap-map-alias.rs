#![allow(dead_code, unused_imports, unused_variables)]
fn main() {
  use std::num::ParseIntError;
  use std::result;
  
  type Result<T> = result::Result<T, ParseIntError>;
  
  fn double_number(number_str: &str) -> Result<i32> {
      unimplemented!();
  }
}
