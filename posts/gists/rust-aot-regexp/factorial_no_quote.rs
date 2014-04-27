#![feature(phase)]

#[phase(syntax)] extern crate factorial_no_quote;

fn main() {
    println!("{}", factorial!());
    // Output:
    // 120
}
