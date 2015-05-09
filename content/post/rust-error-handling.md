+++
date = "2015-05-06T18:22:00-04:00"
draft = true
title = "Error Handling in Rust"
author = "Andrew Gallant"
url = "rust-error-handling"

[blackfriday]
plainIdAnchors = true
+++

Like most programming languages, Rust encourages the programmer to handle
errors in a particular way. Generally speaking, error handling is divided into
two broad categories: exceptions and return values. Rust opts for return
values.

In this article, I intend to provide a comprehensive treatment of how to deal
with errors in Rust. I will cover error handling in libraries, where it is the
library's responsibility to pass errors on to the caller. I will also cover
error handling in command line programs, where it is the program's
responsibility to present errors to the user in a clean and predictable manner.

When done naively, error handling in Rust can be verbose and annoying. This
article will explore those stumbling blocks and demonstrate how to use the
standard library to make error handling concise and ergonomic.

**Target audience**: Those new to Rust that don't know its error handling
idioms yet.

<!--more-->


### Brief notes

All code samples in this post compile with Rust `1.0.0-beta.4`. They should
continue to work as Rust 1.0 stable is released.

All code can be found and compiled in
[my blog's repository](https://github.com/BurntSushi/blog/tree/master/code/rust-error-handling).

The [Rust Book](http://doc.rust-lang.org/1.0.0-beta.4/book/)
has a [section on error
handling](http://doc.rust-lang.org/1.0.0-beta.4/book/error-handling.html).
It gives a very brief overview, but doesn't (yet) go into enough detail,
particularly when working with some of the more recent additions to the
standard library.


### Run the code!

If you'd like to run any of the code samples below, then the following should
work:

{{< highlight sh "classprefix=pyg-" >}}
$ git clone git://github.com/BurntSushi/blog
$ cd blog/code/rust-error-handling
$ cargo run --bin NAME-OF-CODE-SAMPLE [ args ... ]
{{< /highlight >}}

Each code sample is labeled with its name.


### Table of Contents

* [The Basics](#the-basics)


### The Bull in a China Shop

I like to think of error handling as using *case analysis* to determine whether
a computation was successful or not. As we will see, the key to ergnomic error
handling is reducing the amount of explicit case analysis the programmer has to
do while keeping code composable.

Keeping code composable is important, because without that requirement, we
could [`panic`](http://doc.rust-lang.org/std/macro.panic!.html) whenever we
come across something unexpected. (`panic` causes the current task to unwind,
and in most cases, the entire program aborts.) Here's an example:

{{< code "rust" "rusterrorhandling" "panic-simple" >}}

(If you like, it's easy to [run this code](#run-the-code).)

If you try running this code, the program will crash with a message like this:

{{< highlight sh "classprefix=pyg-" >}}
thread '<main>' panicked at 'Invalid number: 11', src/bin/panic-simple.rs:5
{{< /highlight >}}

Here's another example that is slightly less contrived. A program that accepts
an integer as an argument, doubles it and prints it.

{{< code-rust "rusterrorhandling" "unwrap-double" >}}
use std::env;

fn main() {
    let mut argv = env::args();
    let arg: String = argv.nth(1).unwrap(); // error 1
    let n: i32 = arg.parse().unwrap(); // error 2
    println!("{}", 2 * n);
}

// $ cargo run --bin 01-unwrap 5
// 10
{{< /code-rust >}}

If you give this program zero arguments (error 1) or if the first argument
isn't an integer (error 2), the program will panic just like in the first
example.

I like to think of this style of error handling as similar to a bull running
through a China shop. The bull will get to where it wants to go, but it will
trample everything in the process.

