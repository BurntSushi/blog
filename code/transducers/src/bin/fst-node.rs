#![allow(dead_code, unused_imports, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    use fst::raw::{Builder, Fst};
  
  let mut builder = Builder::memory();
  builder.insert("bar", 1).unwrap();
  builder.insert("baz", 2).unwrap();
  builder.insert("foo", 3).unwrap();
  let fst_bytes = try!(builder.into_inner());
  let fst = try!(Fst::from_bytes(fst_bytes));
  
  // Get the root node of this FST.
  let root = fst.root();
  
  // Print the transitions out of the root node.
  // outputs "b" followed by "f"
  for transition in root.transitions() {
      println!("{}", transition.inp as char);
  }
  
  // Find the position of a transition based on the input.
  let i = root.find_input(b'b').unwrap();
  
  // Get the transition.
  let trans = root.transition(i);
  
  // Get the node that the transition points to.
  let node = fst.node(trans.addr);
  
  // And so on...
    Ok(())
}

fn main() {
    main2().unwrap();
}
