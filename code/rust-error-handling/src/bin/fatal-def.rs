#![allow(dead_code, unused_imports, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    macro_rules! fatal {
      ($($tt:tt)*) => {{
          use std::io::Write;
          writeln!(&mut ::std::io::stderr(), $($tt)*).unwrap();
          ::std::process::exit(1)
      }}
  }
    Ok(())
}

fn main() {
    main2().unwrap();
}
