#![feature(phase)]
#[phase(syntax)] extern crate factorial_arg;

fn main() {
    println!("{}", factorial!(10u));
    // Output:
    // 3628800
}
