#![allow(dead_code, unused_imports, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    use std::error::Error;
  use std::fs;
  use std::io;
  use std::num;
  
  // We have to jump through some hoops to actually get error values.
  let io_err: io::Error = io::Error::last_os_error();
  let parse_err: num::ParseIntError = "not a number".parse::<i32>().unwrap_err();
  
  // OK, here are the conversions.
  let err1: Box<Error> = From::from(io_err);
  let err2: Box<Error> = From::from(parse_err);
    Ok(())
}

fn main() {
    main2().unwrap();
}
