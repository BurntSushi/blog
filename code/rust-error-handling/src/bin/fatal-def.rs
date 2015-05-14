#![allow(dead_code, unused_imports, unused_variables)]
fn main() {
  macro_rules! fatal {
      ($($tt:tt)*) => {{
          use std::io::Write;
          writeln!(&mut ::std::io::stderr(), $($tt)*).unwrap();
          ::std::process::exit(1)
      }}
  }
}
