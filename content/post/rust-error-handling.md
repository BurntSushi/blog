+++
date = "2015-05-06T18:22:00-04:00"
draft = true
title = "Error Handling in Rust"
author = "Andrew Gallant"
url = "rust-error-handling"
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


### Brief notes on format

All code samples in this post compile with Rust `1.0.0-beta.4`. They should
continue to work as Rust 1.0 stable is released.

All code can be found and compiled in
[my blog's repository](https://github.com/BurntSushi/blog/tree/master/code/rust-error-handling).

{{< code "rust" "rusterrorhandling" "01-unwrap" >}}

