+++
date = "2018-09-05T12:00:00-04:00"
title = "ripgrep as a library"
author = "Andrew Gallant"
url = "libripgrep"

[blackfriday]
plainIdAnchors = true
+++

[ripgrep](https://github.com/BurntSushi/ripgrep)
is a very fast grep-like tool for searching files. It specializes in code
search by filtering out files in your `.gitignore` automatically and comes
jammed pack with a lot of features like file type filtering, Unicode support,
opt-in PCRE2 support, UTF-16/GBK/Shift_JIS support, compressed file search and
multi-line search.

Since ripgrep was released about two years ago, its core application code
has been slowly moving into libraries so that others can benefit from them.
Applications that are using code that was born in ripgrep core include, but are
not limited to,
[rustc](https://github.com/rust-lang/rust),
[Cargo](https://github.com/rust-lang/cargo),
[Tokei](https://github.com/Aaronepower/tokei),
[loc](https://github.com/cgag/loc),
[fd](https://github.com/sharkdp/fd),
[Pijul](https://nest.pijul.com/pijul_org/pijul),
[fastmod](https://github.com/facebookincubator/fastmod),
[Cobalt](https://github.com/cobalt-org/cobalt.rs),
[watchexec](https://github.com/watchexec/watchexec)
and
[bingrep](https://github.com/m4b/bingrep).

Until recently, ripgrep core included most of the code used to execute a regex
search and print the results. As of the latest release, all of the searching
and printing logic has been factored out into a group of libraries that can
either be used individually or as part of a single facade defined in the
[`grep`](https://docs.rs) library.

This article will explore the libraries that make up ripgrep, with a specific
focus on the libraries that implement fast regex searching. We will see just
how easy it is to *build your own grep*.

**Target audience**: Rust programmers.

<!--more-->

## Table of contents

* [Overview](#overview)
* [Basics](#basics)
* [Better UX](#better-ux)
* [The Sink Trait](#the-sink-trait)
* [Printing](#printing)
* [Filtering](#filtering)
* [Parallelism](#parallelism)


## Overview

When ripgrep was first released, its core included a lot of stuff:

* [A custom implementation of globbing.](https://github.com/BurntSushi/ripgrep/blob/0.1.17/src/glob.rs)
* [A gitignore parser and matcher.](https://github.com/BurntSushi/ripgrep/blob/0.1.17/src/gitignore.rs)
* [Filtering logic for all ignore-related files.](https://github.com/BurntSushi/ripgrep/blob/0.1.17/src/ignore.rs)
* [A recursive directory walker for efficient filtering.](https://github.com/BurntSushi/ripgrep/blob/0.1.17/src/walk.rs)
* [A file type matcher.](https://github.com/BurntSushi/ripgrep/blob/0.1.17/src/types.rs)
* [A contorted attempt to provide cross platform terminal colors.](https://github.com/BurntSushi/ripgrep/blob/0.1.17/src/out.rs)
* [An incremental search implementation with ~constant memory overhead.](https://github.com/BurntSushi/ripgrep/blob/0.1.17/src/search_stream.rs)
* [A memory map search implementation.](https://github.com/BurntSushi/ripgrep/blob/0.1.17/src/search_buffer.rs)
* [A printer for showing search results.](https://github.com/BurntSushi/ripgrep/blob/0.1.17/src/printer.rs)

Every single one of these things is now in a library. Here's a non-exhaustive
list of libraries that were born from the code linked above:

* [globset](https://docs.rs/globset) provides a more robust implementation of
  globbing than the standard Rust `glob` library, including non-UTF-8 support
  and `{foo,bar}` globbing syntax.
* [ignore](https://docs.rs/ignore) provides sophisticated file filtering logic
  with a parallel recursive directory iterator. This library knows how to parse
  and apply `gitignore` rules.
* [termcolor](https://docs.rs/termcolor) provides a cross platform and
  efficient means to write colors to a terminal that works well in
  multi-threaded environments.
* [grep](https://docs.rs/grep) provides a facade over a variety of pluggable
  libraries that implement fast regex search and printing.

While `globset`, `ignore` and `termcolor` are all important to ripgrep's user
facing features, `grep` is what provides the core functionality: fast search.
`grep` is itself a composition of loosely coupled libraries:

* [grep-matcher](https://docs.rs/grep-matcher) defines the
  [`Matcher`](https://docs.rs/grep-matcher/*/grep_matcher/trait.Matcher.html)
  interface for describing common operations provided by regular expressions.
  This includes finding matches, extracting capture groups and performing
  replacements. This library permits the use of any regex implementation.
* [grep-regex](https://docs.rs/grep-regex) provides an implementation of
  `grep-matcher`'s interface using
  [Rust's regex library](https://docs.rs/regex). This is also where a host
  of optimizations are applied at the regex level to make line oriented
  searching fast.
* [grep-pcre2](https://docs.rs/grep-pcre2) provides an implementation of
  `grep-matcher`'s interface using the [PCRE2](https://www.pcre.org/) regex
  engine. Unlike Rust's regex library, PCRE2 provides several fancy features
  such as look-around and back-references at the cost of slower search speed in
  the worst case.
* [grep-searcher](https://docs.rs/grep-searcher) provides the high level
  implementation of searching files and reporting their results. This includes
  features such as counting lines, setting the line terminator, inverting
  matches, reporting matches across multiple lines, reporting contextual
  lines, binary data detection, transcoding and even whether or not to use
  memory maps. This library defines the
  [`Sink`](https://docs.rs/grep-searcher/*/grep_searcher/trait.Sink.html)
  interface, which describes how callers must behave to receive search
  results.
* [grep-printer](https://docs.rs/grep-printer) provides a few implementations
  of the aforementioned `Sink` trait. This includes a JSON output format in
  addition to the standard grep-like format with support for coloring,
  multi-line result handling, search & replace and various other formatting
  tweaks.
* [grep-cli](https://docs.rs/grep-cli) provides cross platform convenience
  routines with user friendly error messages for common operations performed in
  grep-like command line applications. This includes checking for a tty,
  inferring a stdout buffering strategy and reading from the output of
  other processes.

While some use cases may call for using each of the above libraries
individually, most uses should probably use
[`grep`](https://docs.rs/grep)
itself, which re-exports all of the above libraries. Each library is
re-exported without its `grep-` prefix. For example, `use grep::searcher::Sink`
will bring the `Sink` trait into scope. (Note that `grep-pcre2` is only
included when the `pcre2` feature is enabled, which is not the default.)


## Basics

To get us started, let's take a look at a very simple example of a grep-like
tool. We'll dissect what's going on in the code and iterate on improvements by
using more of the functionality offered by `grep`.

If you'd like to follow along and try the example on your own machine,
first setup a new Cargo project in any directory (see the
[Rust install instructions](https://www.rust-lang.org/install.html)
if you don't have Cargo on your machine):

```
$ cargo new --bin simplegrep
$ cd simplegrep
```

Now make sure your `Cargo.toml` looks something like this (in particular, with
the added dependencies):

```
$ cat Cargo.toml
[package]
name = "simplegrep"
version = "0.1.0"
authors = ["Andrew Gallant <jamslam@gmail.com>"]

[dependencies]
grep = "0.2"
walkdir = "2"
```

Now copy the example below into `src/main.rs`, replacing whatever was there:

{{< high rust "https://github.com/BurntSushi/ripgrep/blob/0.10.0/grep/examples/tutorial-basics-01.rs" >}}
//tutorial-basics-01.rs
extern crate grep;
extern crate walkdir;

use std::error::Error;
use std::io::{self, Write};
use std::process;

use grep::regex::RegexMatcher;
use grep::searcher::Searcher;
use grep::searcher::sinks;
use walkdir::WalkDir;

fn main() {
    if let Err(err) = try_main() {
        eprintln!("{}", err);
        process::exit(1);
    }
}

fn try_main() -> Result<(), Box<Error>> {
    let mut stdout = io::stdout();
    // We hard code our pattern and what we search to keep this example simple.
    let matcher = RegexMatcher::new_line_matcher("PM_RESUME")?;
    let mut searcher = Searcher::new();

    for result in WalkDir::new("./") {
        let dir_entry = result?;
        if !dir_entry.file_type().is_file() {
            continue;
        }

        let path = dir_entry.path();
        // The use of sinks::Lossy here means "lossily convert matching
        // lines to UTF-8." Invalid UTF-8 bytes are substituted with the
        // Unicode replacement codepoint.
        searcher.search_path(&matcher, path, sinks::Lossy(|lineno, line| {
            // Matching lines always include a line terminator, so we
            // don't need to write one ourselves.
            write!(stdout, "{}:{}:{}", path.display(), lineno, line)?;
            Ok(true)
        }))?;
    }
    Ok(())
}
{{< /high >}}

And finally build the project, change into the `src` directory and run the
command:

```
$ cargo build --release
$ cd src
$ ../target/release/simplegrep
./main.rs:22:    let matcher = RegexMatcher::new_line_matcher("PM_RESUME")?;
```

For future examples, you'll want to repeat the above process (or just reuse
your `simplegrep` project by replacing the source code). If new dependencies
are required, we'll note them. Finally, the full source of most example
programs can be found via the links beneath the code. Alternatively, you may
browse the
[examples on GitHub](https://github.com/BurntSushi/ripgrep/tree/0.10.0/grep/examples).

This article isn't going to cover the basics of Rust, so we won't walk through
every detail of the example above, but let's talk about a few things of note.

Firstly, if you run this program on a large directory, you should be able to
observe that its performance (both in time and memory usage) is comparable to
other search tools known for their speed, such as GNU grep. One such corpus we
can test with is a snapshot of the Linux kernel source:

```
$ git clone --depth 1 git://github.com/BurntSushi/linux
$ cd linux
$ time path/to/simplegrep/target/release/simplegrep | wc -l
17
real    0.622
user    0.207
sys     0.403
$ time grep -arn PM_RESUME | wc -l
17
real    0.852
user    0.502
sys     0.340
```

For the `grep` command, we use the flags `-a`, `-r` and `-n` because they
correspond to the behavior of our `simplegrep` example. Namely, `simplegrep`
does not do binary data detection (`-a`), searches the current directory
recursively (`-r`) and reports line numbers (`-n`). This program might not
look like much, but we can look at the "naive" approach without `grep` to see
what we're getting:

{{< high rust "https://github.com/BurntSushi/ripgrep/blob/0.10.0/grep/examples/tutorial-basics-02.rs" >}}
//tutorial-basics-02.rs
# extern crate regex;
# extern crate walkdir;
#
# use std::error::Error;
# use std::fs::File;
# use std::io::{self, BufRead, Write};
# use std::process;
#
# use regex::bytes::Regex;
# use walkdir::WalkDir;
#
# fn main() {
#     if let Err(err) = try_main() {
#         eprintln!("{}", err);
#         process::exit(1);
#     }
# }

fn try_main() -> Result<(), Box<Error>> {
    let mut stdout = io::stdout();
    let matcher = Regex::new("PM_RESUME")?;

    for result in WalkDir::new("./") {
        let dir_entry = result?;
        if !dir_entry.file_type().is_file() {
            continue;
        }

        let path = dir_entry.path();
        let mut rdr = io::BufReader::new(File::open(path)?);
        let mut line = vec![];
        let mut lineno = 1;
        while rdr.read_until(b'\n', &mut line)? > 0 {
            if matcher.is_match(&line) {
                let line = String::from_utf8_lossy(&line);
                write!(stdout, "{}:{}:{}", path.display(), lineno, line)?;
            }
            lineno += 1;
            line.clear();
        }
    }
    Ok(())
}
{{< /high >}}

Running it reveals it's about twice as slow:

```
$ time simplegrep | wc -l
17
real    1.380
user    1.062
sys     0.309
```

In our "naive" approach, notice that we are not using a buffered reader's
standard
[`lines`](https://doc.rust-lang.org/std/io/trait.BufRead.html#method.lines)
method. This is because the `lines` method requires the input to contain
valid UTF-8, and this is not true for every file in the Linux source tree (even
for files that are purportedly source code). If you did try to use `lines` here,
you'd notice that some content simply cannot be searched and the program would
run even slower because of the UTF-8 validation. In both of the above programs,
we avoid UTF-8 validation by searching the raw bytes directly.

In this specific example, the `grep` crate primarily attains its speed by
doing the same thing that GNU grep does: it avoids parsing each line and
instead searches many lines at once.

Going back to our [original example](#code-tutorial-basics-01), let's briefly
talk about the aspects of `grep` that we're using.

First up is the
[`RegexMatcher`](https://docs.rs/grep-regex/*/grep_regex/struct.RegexMatcher.html).
This represents a wrapper implementation over Rust's regex library for the
[`Matcher`](https://docs.rs/grep-matcher/*/grep_matcher/trait.Matcher.html)
trait in the `grep-matcher` crate. We need this wrapper implementation because
`grep`'s search routines are parameterized over the `Matcher` trait. That is,
the search routines don't know anything about Rust's regex crate, and they can
instead be used with any regex engine.

In order to drive this point home, let's look at the next thing from the `grep`
crate, which is the use of a
[`Searcher`](https://docs.rs/grep-searcher/*/grep_searcher/struct.Searcher.html)
from the `grep-searcher` crate. In particular, we use its
[`search_path`](https://docs.rs/grep-searcher/*/grep_searcher/struct.Searcher.html#method.search_path)
method, which has this type signature:

{{< high rust >}}
pub fn search_path<P, M, S>(
    &mut self,
    matcher: M,
    path: P,
    write_to: S,
) -> Result<(), S::Error>
where P: AsRef<Path>,
      M: Matcher,
      S: Sink,
{
    // implementation elided
}
{{< /high >}}

At a high level, this routine takes a matcher, a file path, a way to report
matches and executes a search. If there was a problem executing a search, then
an error is returned. The type parameters can be a lot to chew on, so let's
break them down:

* `P` refers to any type that can be cheaply converted into a file path.
  This can be a `&Path` itself or a `&str` or a `String`, among others.
* `M` refers to the implementation of `Matcher` that we use. In our example,
  this is instantiated to `RegexMatcher`.
* `S` refers to the implementation of
  [`Sink`](https://docs.rs/grep-searcher/*/grep_searcher/trait.Sink.html),
  which is how search results are reported. In our example, we use
  [`Lossy`](https://docs.rs/grep-searcher/*/grep_searcher/sinks/struct.Lossy.html),
  which is a pre-built implementation of `Sink` that automatically converts
  matches to UTF-8 while substituting invalid UTF-8 bytes with the Unicode
  replacement codepoint. There are
  [other convenience implementations of `Sink`](https://docs.rs/grep-searcher/*/grep_searcher/sinks/index.html)
  provided for you, but of course, you can also implement `Sink` yourself.
* `S::Error` is an associated type on the `Sink` trait that refers to the type
  of error that is used by the `Sink` implementation. In this case, the `Lossy`
  implementation uses `io::Error` from the standard library. Searching tends to
  typically fail on I/O operations, so `io::Error` is a common error type when
  using the `grep` crate.

Now that we've dissected the introductory example, let's move on to making our
command line application a bit more robust with better
<abbr title="User Experience">UX</abbr>. It's OK if all of the pieces don't
quite fit together yet. We'll get more practice with the `Matcher` and `Sink`
traits later.


## Better UX


## The Sink Trait
