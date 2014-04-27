#![feature(phase)]

extern crate regex;
#[phase(syntax)]
extern crate regex_macros;

fn main() {
    let _ = regex!(r"hippo[a-fx-z0-9]+");
}
