#![feature(phase)]
extern crate regex;
#[phase(syntax)] extern crate regex_macros;

fn main() {
    let re = regex!(r"\d{4}-\d{2}-\d{2}");
    println!("{}", re.find("Today's date is 2014-04-21."));
    // Output:
    // Some((16, 26))
}
