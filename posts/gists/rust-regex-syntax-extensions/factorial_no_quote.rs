#![feature(phase)]

#[phase(plugin)] extern crate factorial_no_quote;

fn main() {
    println!("{}", factorial!());
    // Output:
    // 120
}
