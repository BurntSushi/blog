extern crate regex;
use regex::Regex;

fn main() {
    let re = match Regex::new(r"\d{4}-\d{2}-\d{2}") {
        Ok(re) => re,
        Err(err) => fail!("{}", err),
    };
    println!("{}", re.find("Today's date is 2014-04-21."));
    // Output:
    // Some((16, 26))
}
