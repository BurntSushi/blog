#![feature(phase)]

extern crate regex;
#[phase(plugin)]
extern crate regex_macros;

fn main() {
    let _ = regex!(r"hippo[a-fx-z0-9]+");
}
