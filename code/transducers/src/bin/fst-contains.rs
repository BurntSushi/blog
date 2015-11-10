#![allow(dead_code, unused_imports, unused_variables)]
extern crate fst;

use std::error::Error;

fn main2() -> Result<(), Box<Error+Send+Sync>> {
    use fst::raw::Fst;
  
  // The function takes a reference to an Fst and a key and returns true if
  // and only if the key is in the Fst.
  fn contains_key(fst: &Fst, key: &[u8]) -> bool {
      // Start the search at the root node.
      let mut node = fst.root();
      // Iterate over every byte in the key.
      for b in key {
          // Look for a transition in this node for this byte.
          match node.find_input(*b) {
              // If one cannot be found, we can conclude that the key is not
              // in this FST.
              None => return false,
              // Otherwise, we set the current node to the node that the found
              // transition points to. In other words, we "advance" the finite
              // state machine.
              Some(i) => {
                  node = fst.node(node.transition_addr(i));
              }
          }
      }
      // After we've exhausted the key to look up, it is only in the FST if we
      // ended at a final state.
      node.is_final()
  }
    Ok(())
}

fn main() {
    main2().unwrap();
}
