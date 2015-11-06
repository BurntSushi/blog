#![allow(dead_code, unused_imports, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    let string: String = From::from("foo");
  let bytes: Vec<u8> = From::from("foo");
  let cow: ::std::borrow::Cow<str> = From::from("foo");
    Ok(())
}

fn main() {
    main2().unwrap();
}
