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
multi-line search. Most of the standard grep-like flags you've become familiar
with are also available.

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
  implementation uses `io::Error` from the standard library. When searching
  fails, it tends to typically be on I/O operations, so `io::Error` is a common
  error type when using the `grep` crate.

Now that we've dissected the introductory example, let's move on to making
our command line application a bit more robust with better <abbr title="User
Experience">UX</abbr>. It's okay if it's not clear how all of the pieces fit
together yet. We'll get more practice with the `Matcher` and `Sink` traits
later.


## Better UX

The
[initial example](#code-tutorial-basics-01)
is brief and fairly easy to understand, but brevity comes with a cost in this
case. In particular, there are a few user experience problems with how the
program works:

* The pattern (`PM_RESUME`) is hard coded.
* The files to search (the current directory) is hard coded.
* If a file does not end with a new line and its last line is part of a match,
  the output of our program itself won't end with a new line.
* We use `io::stdout()`, which is line buffered. While this works well when
  the number of matches are low, it can adverse impact performance when the
  number of matches is high due to the frequency of flushing the line buffer.
* An error searching any one file will cause the entire program to quit.
  Moreover, the error message won't include the path to the problematic file.
* If our program is used in a shell pipeline and the receiving program closes
  our program's pipe, then instead of exiting gracefully, it will quit with
  an error message and a non-zero exit code.

Depending on what problem you're trying to solve, you might not actually care
about any of the above UX problems, or you might care about all of them. In the
case of ripgrep, we definitely care about all of them, so we're going to cover
some basic solutions to these problems.

Let's get the hard coded pattern and file paths out of the way first. Since we
want to keep this simple, we'd like to require one pattern as our first
position argument and then zero or more paths (files or directories) to search.
If none are given, we can use the current directory by default. In sum, our
usage description looks something like this:

```
$ simplegrep <pattern> [<path> ...]
```

Let's take a look at a simple function that implements this specification. To
parse command line arguments, we use
[clap](https://docs.rs/clap),
which is the same argument parsing library that ripgrep uses. It's fast, well
maintained and can fit almost any use case. In order to use clap, you'll want
to add `clap = "2"` to the `[dependencies]` section of your `Cargo.toml`.

{{< high rust >}}
/// Parse command line arguments into a single pattern and a list of one or
/// more paths to search.
fn parse_argv() -> Result<(String, Vec<OsString>), Box<Error>> {
    use clap::{App, Arg};

    let args = App::new("A simple grep-like example")
        .version("0.0.1")
        .arg(Arg::with_name("pattern").required(true))
        .arg(Arg::with_name("path").multiple(true))
        .get_matches();
    let pattern = cli::pattern_from_os(args.value_of_os("pattern").unwrap())?;
    let paths = args.values_of_os("path")
        .map(|osstrs| osstrs.map(|osstr| osstr.to_os_string()).collect())
        .unwrap_or(vec![OsString::from("./")]);
    Ok((pattern.to_string(), paths))
}
{{< /high >}}

With this bit of code, we get argument validation in addition to helpful
messages for the `--version` and `--help` flag. One thing worth calling out
here is the use of the
[`cli::pattern_from_os`](https://docs.rs/grep-cli/*/grep_cli/fn.pattern_from_os.html)
function from the `grep-cli` crate. The purpose of this function is to give
helpful error messages if the pattern given isn't valid UTF-8. For example:

```
$ simplegrep $(echo -e "a\x80z")
found invalid UTF-8 in pattern at byte offset 1 (use hex escape sequences to match arbitrary bytes in a pattern, e.g., \xFF): 'a\x80z'
```

We can adapt our `try_main` routine from our
[example](#code-tutorial-basics-01)
fairly easily by calling `parse_argv` and recursively traversing each path
given. (If a path is a normal file, then the `WalkDir` iterator will return
just that file, which simplifies case analysis.)

{{< high rust >}}
fn try_main() -> Result<(), Box<Error>> {
    let mut stdout = io::stdout();
    let (pattern, paths) = parse_argv()?;
    let matcher = RegexMatcher::new_line_matcher(&pattern)?;
    let mut searcher = Searcher::new();

    for path in paths {
        for result in WalkDir::new(path) {
            let dir_entry = result?;
            if !dir_entry.file_type().is_file() {
                continue;
            }

            let path = dir_entry.path();
            searcher.search_path(&matcher, path, sinks::Lossy(|lineno, line| {
                write!(stdout, "{}:{}:{}", path.display(), lineno, line)?;
                Ok(true)
            }))?;
        }
    }
    Ok(())
}
{{< /high >}}

The next thing we'd like to tackle here is to make sure the output of our
simple grep program always ends with a new line, even if the file it is
searching does not. The reason for this behavior is to avoid situations like
this:

```
$ echo -n foo > no-new-line
$ simplegrep foo no-new-line
no-new-line:1:foo$
```

Notice where my prompt ends up: right after the contents of `no-new-line`
instead of on a line of its own. Note that for users of more enlightened
shells (such as `zsh`), this is less of an issue:

```
$ scratch foo no-new-line
no-new-line:1:foo%
$
```

Nevertheless, we endeavor to fix it. We only need to focus on the small bit of
code that prints matching lines:

{{< high rust >}}
searcher.search_path(&matcher, path, sinks::Lossy(|lineno, line| {
    write!(stdout, "{}:{}:{}", path.display(), lineno, line)?;
    Ok(true)
}))?;
{{< /high >}}

To fix our bug, we simply need to check whether the line ends with a line
terminator, and if not, print one:

{{< high rust >}}
searcher.search_path(&matcher, path, sinks::Lossy(|lineno, line| {
    write!(stdout, "{}:{}:{}", path.display(), lineno, line)?;
    if !line.ends_with('\n') {
        stdout.write_all(&[b'\n'])?;
    }
    Ok(true)
}))?;
{{< /high >}}

Before moving to better error handling, we'd like to quickly fix our stdout
buffering bug. As a reminder, the problem here is that the standard library's
`io::stdout()` function returns a line buffered
[`Stdout`](https://doc.rust-lang.org/std/io/struct.Stdout.html).
It turns out that this is what you want most of the time. In particular, if
your command line program is printing to a terminal directly, then using line
buffering is important because each line is printed as it becomes available.
An alternative buffering strategy, called "block" buffering, will store
potentially many lines in memory before finally sending them to stdout. This
is typically ideal when redirecting output to another command or file.

A naive solution to this problem is to simply wrap stdout in a buffer:

{{< high rust >}}
let mut stdout = io::BufReader::new(io::stdout());
{{< /high >}}

This works well enough and will fix our performance problem, but we've
increased the latency at which lines are printed to a terminal. Therefore, we'd
instead like to choose our buffering strategy based on whether there is a
terminal attached to stdout or not. The `stdout` method in `grep-cli` will do
just that. All we need to do is use it:

{{< high rust >}}
let mut stdout = cli::stdout(termcolor::ColorChoice::Never);
{{< /high >}}

We do need to specify our color preference, and for now, we simply disable it.
This will require adding `termcolor = "1"` to the `[dependencies]` section of
your `Cargo.toml`. We will talk a bit more about color later in this tutorial.

The last bits we want to take care of in this section relate to error handling.
Namely, if we get an error while searching a file, we want to show that error
to the end user, but then continue searching. Our current program doesn't do
this, because of the use of `?`, which will stop the current function and
return the error immediately.

To see how our program fails, try these two examples in the Linux source code
checkout from the previous section:

```
$ simplegrep PM_RESUME | head -n1
./arch/x86/kernel/apm_32.c:486: { APM_RESUME_DISABLED,  "Resume timer disabled" },
Broken pipe (os error 32)

$ touch no-access
$ chmod 000 no-access
$ simplegrep PM_RESUME
...
Permission denied (os error 13)
```

Your output might differ slightly depending on the order of files traversed,
but the failure modes should be the same. In the first case, we get an error
instead of quitting gracefully. In the second case, our program stops without
searching subsequent files.

There are two places in our code that need to be fixed. The first is getting
our directory entry:

{{< high rust >}}
for result in WalkDir::new(path) {
    let dir_entry = result?;
{{< /high >}}

In particular, `result` in this code is a
`Result<walkdir::DirEntry, walkdir::Error>`. `walkdir` itself permits
continuing traversal of the directory even if there's an error by requiring the
caller to handle the error. Here's one way to do that:

{{< high rust >}}
for result in WalkDir::new(path) {
    let dir_entry = match result {
        Ok(dir_entry) => dir_entry,
        Err(err) => {
            // This error from walkdir includes the file path.
            eprintln!("{}", err);
            continue;
        }
    };
{{< /high >}}

The second case to handle is our search routine itself. Namely, it can
produce an error if there was a problem opening or reading a file. It can also
produce other kinds of errors from the matcher, but since we're using Rust's
regex engine, such errors are impossible. We handle this error similarly, but
we're careful to check for a pipe error:

{{< high rust >}}
let path = dir_entry.path();
let sink = sinks::Lossy(|lineno, line| {
    write!(stdout, "{}:{}:{}", path.display(), lineno, line)?;
    if !line.ends_with('\n') {
        stdout.write_all(&[b'\n'])?;
    }
    Ok(true)
});
if let Err(err) = searcher.search_path(&matcher, path, sink) {
    // If writing to stdout results in a broken pipe, then we
    // should stop everything immediately and quit gracefully.
    if err.kind() == io::ErrorKind::BrokenPipe {
        return Ok(());
    }
    // Otherwise, report the error and move on.
    eprintln!("{}: {}", path.display(), err);
    continue;
}
{{< /high >}}

If you re-compile your program and try the examples above that exhibited bad
failure modes, then they should be fixed!

For completeness, here's our fully revised program:

{{< high rust "https://github.com/BurntSushi/ripgrep/blob/0.10.0/grep/examples/tutorial-ux-01.rs" >}}
//tutorial-ux-01.rs
extern crate clap;
extern crate grep;
extern crate termcolor;
extern crate walkdir;

use std::error::Error;
use std::ffi::OsString;
use std::io::{self, Write};
use std::process;

use grep::cli;
use grep::regex::RegexMatcher;
use grep::searcher::Searcher;
use grep::searcher::sinks;
use termcolor::ColorChoice;
use walkdir::WalkDir;

fn main() {
    if let Err(err) = try_main() {
        eprintln!("{}", err);
        process::exit(1);
    }
}

fn try_main() -> Result<(), Box<Error>> {
    let mut stdout = cli::stdout(ColorChoice::Never);
    let (pattern, paths) = parse_argv()?;
    let matcher = RegexMatcher::new_line_matcher(&pattern)?;
    let mut searcher = Searcher::new();

    for path in paths {
        for result in WalkDir::new(path) {
            let dir_entry = match result {
                Ok(dir_entry) => dir_entry,
                Err(err) => {
                    // This error from walkdir includes the file path.
                    eprintln!("{}", err);
                    continue;
                }
            };
            if !dir_entry.file_type().is_file() {
                continue;
            }

            let path = dir_entry.path();
            let sink = sinks::Lossy(|lineno, line| {
                write!(stdout, "{}:{}:{}", path.display(), lineno, line)?;
                if !line.ends_with('\n') {
                    stdout.write_all(&[b'\n'])?;
                }
                Ok(true)
            });
            if let Err(err) = searcher.search_path(&matcher, path, sink) {
                // If writing to stdout results in a broken pipe, then we
                // should stop everything immediately and quit gracefully.
                if err.kind() == io::ErrorKind::BrokenPipe {
                    return Ok(());
                }
                // Otherwise, report the error and move on.
                eprintln!("{}: {}", path.display(), err);
                continue;
            }
        }
    }
    Ok(())
}

/// Parse command line arguments into a single pattern and a list of one or
/// more paths to search.
fn parse_argv() -> Result<(String, Vec<OsString>), Box<Error>> {
    use clap::{App, Arg};

    let args = App::new("A simple grep-like example")
        .version("0.0.1")
        .arg(Arg::with_name("pattern").required(true))
        .arg(Arg::with_name("path").multiple(true))
        .get_matches();
    let pattern = cli::pattern_from_os(args.value_of_os("pattern").unwrap())?;
    let paths = args.values_of_os("path")
        .map(|osstrs| osstrs.map(|osstr| osstr.to_os_string()).collect())
        .unwrap_or(vec![OsString::from("./")]);
    Ok((pattern.to_string(), paths))
}
{{< /high >}}


## The Sink Trait

The
[`Sink` trait](https://docs.rs/grep-searcher/*/grep_searcher/trait.Sink.html)
is one of the two central traits provided by `grep` (the other being the
`Matcher` trait). Namely, the `Sink` trait is the interface by which results
from the searcher are passed back to the caller.

Let's pop up a level for a minute here to give context on the design of the
`Sink` trait. In the Rust ecosystem, idiomatic code tends to use iterators, or
more precisely, implementations of the
[`Iterator` trait](https://doc.rust-lang.org/std/iter/trait.Iterator.html)
in the standard library. Iterators in Rust are more specifically referred to
as *external iterators*, which means that the user of the iterator determines
when to advice to the next step. In contrast, *internal iterators* inverts the
flow of control such that the iterator itself determines when to take the
next step, not the caller. Typically, an internal iterator asks the caller to
provide a closure, and that closure is executed for each step of the iterator.
The caller doesn't get control over the flow of the program until the iterator
stops and returns. Typically, a closure can signal to quit iteration early.
Another critical downside of internal iterators is that they can be terribly
difficult to convert to external iterators, but it is usually very easy to
convert external iterators to internal iterators. Thus, internal iteration has
a type of poisoning effect: once you use it, it's hard to escape it.

In Rust, external iterators are generally preferred whenever possible, because
it keeps the flow of control with the caller and avoids callback oriented code,
which some regard as ghoulish. However, the `grep` crate specifically uses
internal iteration for a couple good reasons, and they revolve around the
`Matcher` trait:

* There exist some regex engines that only provide internal iterators to
  traverse matches in a string. If `grep` required the use of external
  iterators, it would effectively rule out such regex engines from being
  used. (An example of such a regex engine is
  [Hyperscan](https://github.com/intel/hyperscan).
* This one is admittedly a bit more wishy washy, but with Rust's current type
  system, it appears difficult to design a generic regular expression trait
  that uses external iteration. In contrast, internal iteration is nearly
  trivial and doesn't use any sophisticated type system shenanigans.

With all that out of the way, let's take a look at the trait's methods. In
particular, it defines five methods and you are only required to implement
one of them. Each method is called at different points in the search, along
with information specific to that method (like, say, the bytes of the line that
matched) and the
[`Searcher`](https://docs.rs/grep-searcher/*/grep_searcher/struct.Searcher.html)
itself, which permits callers to query the configuration of the searcher. Each
method may return an error, in which case, the search is stopped immediately
and the error is propagated back up to the caller that invoked the searcher.
If an error doesn't occur, then each method returns a `bool` indicating whether
to continue the search or not.

Here are the methods and a brief description of each:

* The
  [`matched`](https://docs.rs/grep-searcher/*/grep_searcher/trait.Sink.html#tymethod.matched)
  method is the one method you must provide, and it defines the
  response to each match reported by the searcher. Each match is at most one
  line long, unless multi-line mode is enabled, in which case, a match may
  span multiple lines. A match is described by
  [`SinkMatch`](https://docs.rs/grep-searcher/*/grep_searcher/struct.SinkMatch.html),
  which includes the raw bytes of the match and things like the line number,
  if the searcher is told to count lines (which is the default).
* The
  [`context`](https://docs.rs/grep-searcher/*/grep_searcher/trait.Sink.html#method.context)
  method is an optional method that is called whenever a contextual line is
  found. Contextual lines are only reported when at least one of
  [`after_context`](https://docs.rs/grep-searcher/*/grep_searcher/struct.SearcherBuilder.html#method.after_context),
  [`before_context`](https://docs.rs/grep-searcher/*/grep_searcher/struct.SearcherBuilder.html#method.before_context)
  or
  [`passthru`](https://docs.rs/grep-searcher/*/grep_searcher/struct.SearcherBuilder.html#method.passthru)
  is set via the `SearcherBuilder`. Contextual information is described by
  [`SinkContext`](https://docs.rs/grep-searcher/*/grep_searcher/struct.SinkContext.html),
  which resembles `SinkMatch`.
* The
  [`context_break`](https://docs.rs/grep-searcher/*/grep_searcher/trait.Sink.html#method.context_break)
  method is called whenever a non-contiguous break is found between contextual
  lines. This is often used to implement a `--` divider between blocks of
  context.
* The
  [`begin`](https://docs.rs/grep-searcher/*/grep_searcher/trait.Sink.html#method.begin)
  method is called before starting a search. This permits the caller to
  perform one-time per-search initialization, or to even decide to quit the
  search before starting based on some criteria.
* The
  [`finish`](https://docs.rs/grep-searcher/*/grep_searcher/trait.Sink.html#method.finish)
  method is called after the completion of a search. This permits callers to
  execute clean-up code or do things like gather statistics. Note that `finish`
  is **not** called if an error occurred anywhere during the search, but it is
  still called if one of the other methods decided to quit the search early
  by returning `false`.

The best way to learn all of this is to try writing an implementation of
`Sink`. To motivate this, let's take our
[last program](#code-tutorial-ux-01)
and observe something interesting: it is much slower than GNU grep when the
number of matches it needs to print is very high. Let's take a look by
searching for `.`, which matches every non-empty line. It's a bit of a
ridiculous use case, but it lets us test the extremes easily.

```
$ time grep -arn . ./ | wc -l
22722720

real    5.119
user    3.990
sys     1.113

$ time simplegrep . ./ | wc -l
22722717

real    9.868
user    8.810
sys     1.034
```

(If you're wondering why the counts are different, it seems that it's likely
due to a bug in GNU grep, although that's hard to say without knowing the
intended behavior. In particular, even though my locale is set to
`en_US.UTF-8`, GNU grep seems to permit `.` to match the UTF-8 encoding of
surrogate codepoints, which are technically not allowed in valid UTF-8.)

What's causing the performance issue? If you take a profiler to the program,
then one obvious area is the UTF-8 conversion happening in `sink::Lossy`. If
you recall, the `sink::Lossy` implementation of `Sink` will take the matching
bytes and convert them to a `&str`, but will replace invalid UTF-8 bytes with
the Unicode replacement codepoint. In Rust, Unicode strings are much easier to
work with than a `&[u8]`, which is what motivates this type of conversion.
Moreover, matches are typically rare, so doing some extra work on just the
matching lines is typically acceptable. In this case, where most lines are
matches, we really want to reduce that work.

One approach we can take here to improve things is to write our own
implementation of `Sink` that does not perform any UTF-8 conversion, and simply
writes the raw bytes to stdout. The implementation isn't too much different
than our `sink::Lossy` code:

{{< high rust "https://github.com/BurntSushi/ripgrep/blob/0.10.0/grep/examples/tutorial-sink-01.rs" >}}
//tutorial-sink-01.rs
# extern crate clap;
# extern crate grep;
# extern crate termcolor;
# extern crate walkdir;
#
# use std::error::Error;
# use std::ffi::OsString;
# use std::io;
# use std::process;
#
# use grep::cli;
# use grep::regex::RegexMatcher;
# use grep::searcher::{Searcher, Sink, SinkMatch};
# use termcolor::ColorChoice;
# use walkdir::WalkDir;
#
# fn main() {
#     if let Err(err) = try_main() {
#         eprintln!("{}", err);
#         process::exit(1);
#     }
# }
#
# fn try_main() -> Result<(), Box<Error>> {
#     let mut stdout = cli::stdout(ColorChoice::Never);
#     let (pattern, paths) = parse_argv()?;
#     let matcher = RegexMatcher::new_line_matcher(&pattern)?;
#     let mut searcher = Searcher::new();
#
#     for path in paths {
#         for result in WalkDir::new(path) {
#             let dir_entry = match result {
#                 Ok(dir_entry) => dir_entry,
#                 Err(err) => {
#                     // This error from walkdir includes the file path.
#                     eprintln!("{}", err);
#                     continue;
#                 }
#             };
#             if !dir_entry.file_type().is_file() {
#                 continue;
#             }
#
#             let path = dir_entry.path();
#             let sink = FastSink { path: path, wtr: &mut stdout };
#             if let Err(err) = searcher.search_path(&matcher, path, sink) {
#                 if err.kind() == io::ErrorKind::BrokenPipe {
#                     return Ok(());
#                 }
#                 eprintln!("{}: {}", path.display(), err);
#                 continue;
#             }
#         }
#     }
#     Ok(())
# }

struct FastSink<'p, W> {
    path: &'p ::std::path::Path,
    wtr: W,
}

impl<'p, W: io::Write> Sink for FastSink<'p, W> {
    // Every Sink implementation must specify the type of error that can be
    // returned. For most uses, an io::Error is just fine. You may use a
    // custom error, although, it must implement the SinkError trait.
    type Error = io::Error;

    fn matched(
        &mut self,
        searcher: &Searcher,
        mat: &SinkMatch,
    ) -> io::Result<bool> {
        // Line numbers are enabled by default on a searcher, so it's OK
        // to unwrap it here.
        let lineno = mat.line_number().expect("line numbers must be enabled");

        // The first part of our line, which contains the path and line number.
        write!(self.wtr, "{}:{}:", self.path.display(), lineno)?;
        // Now write the matching bytes as they were read from the file.
        self.wtr.write_all(mat.bytes())?;

        // Now that we have the searcher value available to us, we can
        // write the line terminator configured on the searcher instead of
        // hard coding our own.
        if !searcher.line_terminator().is_suffix(mat.bytes()) {
            self.wtr.write_all(searcher.line_terminator().as_bytes())?;
        }
        Ok(true)
    }
}

# /// Parse command line arguments into a single pattern and a list of one or
# /// more paths to search.
# fn parse_argv() -> Result<(String, Vec<OsString>), Box<Error>> {
#     use clap::{App, Arg};
#
#     let args = App::new("A simple grep-like example")
#         .version("0.0.1")
#         .arg(Arg::with_name("pattern").required(true))
#         .arg(Arg::with_name("path").multiple(true))
#         .get_matches();
#     let pattern = cli::pattern_from_os(args.value_of_os("pattern").unwrap())?;
#     let paths = args.values_of_os("path")
#         .map(|osstrs| osstrs.map(|osstr| osstr.to_os_string()).collect())
#         .unwrap_or(vec![OsString::from("./")]);
#     Ok((pattern.to_string(), paths))
# }
{{< /high >}}

In this code snippet, we only include our new implementation of `Sink`. In
order to use it, replace the `let sink = ...;` block from the
[previous example](#code-tutorial-ux-01)
with

{{< high rust >}}
let sink = FastSink { path: path, wtr: &mut stdout };
{{< /high >}}

(You can also view the
[full source code for this program on GitHub](https://github.com/BurntSushi/ripgrep/blob/0.10.0/grep/examples/tutorial-sink-01.rs).)

So with this change, how does the performance of our program fair? Let's see:

```
$ time grep -arn . ./ | wc -l
22722720

real    4.924
user    3.930
sys     0.981

$ time simplegrep . ./ | wc -l
22722717

real    6.360
user    5.251
sys     1.094
```

We are better than before, but not quite there yet. If you run your favorite
profiler again, you'll find that our program is still spending a fair bit of
time performing UTF-8 decoding. But how? It's a bit subtle, but the way in
which we are printing our path is actually performing a UTF-8 decoding step.
In particular, a
[path's `display` method](https://doc.rust-lang.org/std/path/struct.Path.html#method.display)
will ensure the contents of the path are valid UTF-8 before printing it. In
our case, we actually want to print the path exactly as we got it, without any
intermediate conversions. Not only is this more correct in the context of our
environment, but it's faster.

Thankfully, the `grep` crate provides a solution even for small performance
bugs such as this. The
[`PrinterPath`](https://docs.rs/grep-printer/*/grep_printer/struct.PrinterPath.html)
type from the `grep-printer` library provides a cross platform way to extract
the raw bytes from a `Path`. On Unix-like systems, this extraction is free,
since Rust's standard library
[exposes the raw bytes](https://doc.rust-lang.org/std/os/unix/ffi/trait.OsStrExt.html)
for us. On Windows, however, we still unfortunately need to lossily encode the
path as UTF-8, for lack of a good alternative. But we can do this once per
file path instead of every time we print it. Making the change to `PrinterPath`
is easy. We simply need to change our construction of `FastSink` from

{{< high rust >}}
let sink = FastSink { path: PrinterPath::new(path), wtr: &mut stdout };
{{< /high >}}

and then tweak our `FastSink` type definition and implementation:

{{< high rust "https://github.com/BurntSushi/ripgrep/blob/0.10.0/grep/examples/tutorial-sink-02.rs" >}}
//tutorial-sink-02.rs
# extern crate clap;
# extern crate grep;
# extern crate termcolor;
# extern crate walkdir;
#
# use std::error::Error;
# use std::ffi::OsString;
# use std::io;
# use std::process;
#
# use grep::cli;
# use grep::printer::PrinterPath;
# use grep::regex::RegexMatcher;
# use grep::searcher::{Searcher, Sink, SinkMatch};
# use termcolor::ColorChoice;
# use walkdir::WalkDir;
#
# fn main() {
#     if let Err(err) = try_main() {
#         eprintln!("{}", err);
#         process::exit(1);
#     }
# }
#
# fn try_main() -> Result<(), Box<Error>> {
#     let mut stdout = cli::stdout(ColorChoice::Never);
#     let (pattern, paths) = parse_argv()?;
#     let matcher = RegexMatcher::new_line_matcher(&pattern)?;
#     let mut searcher = Searcher::new();
#
#     for path in paths {
#         for result in WalkDir::new(path) {
#             let dir_entry = match result {
#                 Ok(dir_entry) => dir_entry,
#                 Err(err) => {
#                     // This error from walkdir includes the file path.
#                     eprintln!("{}", err);
#                     continue;
#                 }
#             };
#             if !dir_entry.file_type().is_file() {
#                 continue;
#             }
#
#             let path = dir_entry.path();
#             let sink = FastSink {
#                 path: PrinterPath::new(path),
#                 wtr: &mut stdout,
#             };
#             if let Err(err) = searcher.search_path(&matcher, path, sink) {
#                 if err.kind() == io::ErrorKind::BrokenPipe {
#                     return Ok(());
#                 }
#                 eprintln!("{}: {}", path.display(), err);
#                 continue;
#             }
#         }
#     }
#     Ok(())
# }

struct FastSink<'p, W> {
    path: PrinterPath<'p>,
    wtr: W,
}

impl<'p, W: io::Write> Sink for FastSink<'p, W> {
    type Error = io::Error;

    fn matched(
        &mut self,
        searcher: &Searcher,
        mat: &SinkMatch,
    ) -> io::Result<bool> {
        // Line numbers are enabled by default on a searcher, so it's OK
        // to unwrap it here.
        let lineno = mat.line_number().expect("line numbers must be enabled");

        // The first part of our line, which contains the path and line number.
        self.wtr.write_all(self.path.as_bytes())?;
        write!(self.wtr, ":{}:", lineno)?;
        self.wtr.write_all(mat.bytes())?;

        // Now that we have the searcher value available to us, we can
        // write the line terminator configured on the searcher instead of
        // hard coding our own.
        if !searcher.line_terminator().is_suffix(mat.bytes()) {
            self.wtr.write_all(searcher.line_terminator().as_bytes())?;
        }
        Ok(true)
    }
}

# /// Parse command line arguments into a single pattern and a list of one or
# /// more paths to search.
# fn parse_argv() -> Result<(String, Vec<OsString>), Box<Error>> {
#     use clap::{App, Arg};
#
#     let args = App::new("A simple grep-like example")
#         .version("0.0.1")
#         .arg(Arg::with_name("pattern").required(true))
#         .arg(Arg::with_name("path").multiple(true))
#         .get_matches();
#     let pattern = cli::pattern_from_os(args.value_of_os("pattern").unwrap())?;
#     let paths = args.values_of_os("path")
#         .map(|osstrs| osstrs.map(|osstr| osstr.to_os_string()).collect())
#         .unwrap_or(vec![OsString::from("./")]);
#     Ok((pattern.to_string(), paths))
# }
{{< /high >}}

How do we fair now?

```
$ time simplegrep . ./ | wc -l
22722717

real    5.075
user    4.091
sys     0.971

$ time grep -arn . ./ | wc -l
22722720

real    4.949
user    3.925
sys     1.011
```

Huzzah! If you take a look at our program under a profiler again, then you
should no longer see any UTF-8 decoding appear. (On my system, it is possible
to shave another whole second off the runtime by using the
[`itoa`](https://docs.rs/itoa)
crate to write line numbers, which avoids going through Rust's standard
formatting machinery. But we shall leave that as an exercise to the reader!)

The only way for us to achieve the best performance in all cases is to roll our
own implementation of `Sink`. The
[pre-made](https://docs.rs/grep-searcher/*/grep_searcher/sinks/index.html)
implementations of `Sink` work just fine in most cases, and indeed, the
[`Bytes`](https://docs.rs/grep-searcher/*/grep_searcher/sinks/struct.Bytes.html)
implementation would let us accomplish our goals in the previous example since
it doesn't do any UTF-8 decoding for you automatically. However, it does
require line numbers to exist, which can be disabled since they require more
work.

Aside from performance, custom `Sink` implementations also give us more
flexibility in how we show our matches. For example, what if we wanted to show
one contextual line before and after each match, just like grep's `--context`
flag? There's a bit of plumbing involved in order to get all the corner cases
right, but the gist of it is to implement the `context` method on our
implementation of `Sink`:

{{< high rust "https://github.com/BurntSushi/ripgrep/blob/0.10.0/grep/examples/tutorial-sink-03.rs" >}}
//tutorial-sink-03.rs
# extern crate clap;
# extern crate grep;
# extern crate termcolor;
# extern crate walkdir;
#
# use std::error::Error;
# use std::ffi::OsString;
# use std::io;
# use std::process;
#
# use grep::cli;
# use grep::matcher::LineTerminator;
# use grep::printer::PrinterPath;
# use grep::regex::RegexMatcher;
# use grep::searcher::{
#     Searcher, SearcherBuilder, Sink, SinkContext, SinkMatch,
# };
# use termcolor::ColorChoice;
# use walkdir::WalkDir;
#
# fn main() {
#     if let Err(err) = try_main() {
#         eprintln!("{}", err);
#         process::exit(1);
#     }
# }

fn try_main() -> Result<(), Box<Error>> {
    let mut stdout = cli::stdout(ColorChoice::Never);
    let (pattern, paths) = parse_argv()?;
    let matcher = RegexMatcher::new_line_matcher(&pattern)?;
    // In order to show contextual lines, we must tell the searcher to find
    // and report them. Without this, contextual lines are ignored even if
    // our Sink implementation defines a `context` method.
    let mut searcher = SearcherBuilder::new()
        .before_context(1)
        .after_context(1)
        .build();
    // A simple piece of state that we used to determine whether to print
    // a `--` separator before showing matches in a file. We don't print it
    // before the first file, but we do print it before each subsequent file.
    let mut has_printed = false;

    for path in paths {
        for result in WalkDir::new(path) {
            let dir_entry = match result {
                Ok(dir_entry) => dir_entry,
                Err(err) => {
                    eprintln!("{}", err);
                    continue;
                }
            };
            if !dir_entry.file_type().is_file() {
                continue;
            }

            let path = dir_entry.path();
            let sink = FastSink {
                path: PrinterPath::new(path),
                wtr: &mut stdout,
                has_printed: &mut has_printed,
                has_this_printed: false,
            };
            if let Err(err) = searcher.search_path(&matcher, path, sink) {
                if err.kind() == io::ErrorKind::BrokenPipe {
                    return Ok(());
                }
                eprintln!("{}: {}", path.display(), err);
                continue;
            }
        }
    }
    Ok(())
}

struct FastSink<'p, W> {
    path: PrinterPath<'p>,
    wtr: W,
    /// Whether anything has been printed to stdout yet.
    has_printed: &'p mut bool,
    /// Whether anything has been printed to stdout for `path`.
    /// When has_printed is true and has_this_printed is false,
    /// we print a context separator.
    has_this_printed: bool,
}

impl<'p, W: io::Write> Sink for FastSink<'p, W> {
    type Error = io::Error;

    fn matched(
        &mut self,
        searcher: &Searcher,
        mat: &SinkMatch,
    ) -> io::Result<bool> {
        if *self.has_printed && !self.has_this_printed {
            self.context_break(searcher)?;
        }

        let lineno = mat.line_number().expect("line numbers must be enabled");
        let lineterm = searcher.line_terminator();
        self.print_line(lineno, mat.bytes(), ':', lineterm)?;
        Ok(true)
    }

    fn context(
        &mut self,
        searcher: &Searcher,
        ctx: &SinkContext,
    ) -> io::Result<bool> {
        if *self.has_printed && !self.has_this_printed {
            self.context_break(searcher)?;
        }

        let lineno = ctx.line_number().expect("line numbers must be enabled");
        let lineterm = searcher.line_terminator();
        self.print_line(lineno, ctx.bytes(), '-', lineterm)?;
        Ok(true)
    }

    fn context_break(&mut self, searcher: &Searcher) -> io::Result<bool> {
        self.wtr.write_all(b"--")?;
        self.wtr.write_all(searcher.line_terminator().as_bytes())?;
        Ok(true)
    }
}

impl<'p, W: io::Write> FastSink<'p, W> {
    /// Print the given line (whether matching or contextual) along with its
    /// line number, field separator and line terminator, if needed.
    ///
    /// The code for printing matching and contextual lines is nearly
    /// identical, so we've factored it out here.
    fn print_line(
        &mut self,
        lineno: u64,
        line: &[u8],
        separator: char,
        lineterm: LineTerminator,
    ) -> io::Result<()> {
        self.has_this_printed = true;
        *self.has_printed = true;

        self.wtr.write_all(self.path.as_bytes())?;
        write!(self.wtr, "{}{}{}", separator, lineno, separator)?;
        self.wtr.write_all(line)?;

        if !lineterm.is_suffix(line) {
            self.wtr.write_all(lineterm.as_bytes())?;
        }
        Ok(())
    }
}

# /// Parse command line arguments into a single pattern and a list of one or
# /// more paths to search.
# fn parse_argv() -> Result<(String, Vec<OsString>), Box<Error>> {
#     use clap::{App, Arg};
#
#     let args = App::new("A simple grep-like example")
#         .version("0.0.1")
#         .arg(Arg::with_name("pattern").required(true))
#         .arg(Arg::with_name("path").multiple(true))
#         .get_matches();
#     let pattern = cli::pattern_from_os(args.value_of_os("pattern").unwrap())?;
#     let paths = args.values_of_os("path")
#         .map(|osstrs| osstrs.map(|osstr| osstr.to_os_string()).collect())
#         .unwrap_or(vec![OsString::from("./")]);
#     Ok((pattern.to_string(), paths))
# }
{{< /high >}}

There are some comments in the code snippet above to explain the additions,
but we otherwise won't go into too much detail here. Instead, the purpose of
showing you this code is to emphasize the following points:

1. Printing logic can quickly become complex, especially when dealing with the
   number of output control options in a tool like ripgrep.
2. Printing _can_ be a performance sensitive part of your application. It
   heavily depends on workloads. For very few matches, the performance of
   printing probably doesn't matter. But for a lot of matches, it does matter,
   and folks generally expect operations like this to work and work well.
   Of all extant search tools that I've tried, only GNU grep, pcre2grep, icgrep
   and ripgrep are able to execute the `.` search correctly and in a reasonable
   amount of time on large inputs. This is only possible because of the
   attention paid to the printing code (among other things).

Indeed, this should help motivate _why_ the `grep-printer` library exists in
the first place. It is one of the most complicated and well tested aspects of
all the libraries in `grep`, and in the next section, we're going to see how
much it can do for you, so long as you're willing to live with standard
grep-like formatting.


## Printing

In this section, we are going to discuss the functionality provided by the
`grep-printer` library. In particular, we will show how to use a printer
instead of hand-rolling your own custom `Sink` implementation. For that, let's
start with our
[example at the end of the UX section](#code-tutorial-ux-01),
which was the last example that used `sinks::Lossy` instead of our custom
`FastSink` implementation. Adapting it to use a pre-built printer is simple:

{{< high rust "https://github.com/BurntSushi/ripgrep/blob/0.10.0/grep/examples/tutorial-printing-01.rs" >}}
//tutorial-printing-01.rs
# extern crate clap;
# extern crate grep;
# extern crate termcolor;
# extern crate walkdir;
#
# use std::error::Error;
# use std::ffi::OsString;
# use std::io;
# use std::process;
#
# use grep::cli;
use grep::printer::Standard;
# use grep::regex::RegexMatcher;
# use grep::searcher::Searcher;
use termcolor::ColorChoice;
# use walkdir::WalkDir;
#
# fn main() {
#     if let Err(err) = try_main() {
#         eprintln!("{}", err);
#         process::exit(1);
#     }
# }

fn try_main() -> Result<(), Box<Error>> {
    let stdout = cli::stdout(ColorChoice::Never);
    let mut printer = Standard::new(stdout);

    let (pattern, paths) = parse_argv()?;
    let matcher = RegexMatcher::new_line_matcher(&pattern)?;
    let mut searcher = Searcher::new();

    for path in paths {
        for result in WalkDir::new(path) {
            let dir_entry = match result {
                Ok(dir_entry) => dir_entry,
                Err(err) => {
                    eprintln!("{}", err);
                    continue;
                }
            };
            if !dir_entry.file_type().is_file() {
                continue;
            }

            let path = dir_entry.path();
            let sink = printer.sink_with_path(&matcher, path);
            if let Err(err) = searcher.search_path(&matcher, path, sink) {
                if err.kind() == io::ErrorKind::BrokenPipe {
                    return Ok(());
                }
                eprintln!("{}: {}", path.display(), err);
                continue;
            }
        }
    }
    Ok(())
}

# /// Parse command line arguments into a single pattern and a list of one or
# /// more paths to search.
# fn parse_argv() -> Result<(String, Vec<OsString>), Box<Error>> {
#     use clap::{App, Arg};
#
#     let args = App::new("A simple grep-like example")
#         .version("0.0.1")
#         .arg(Arg::with_name("pattern").required(true))
#         .arg(Arg::with_name("path").multiple(true))
#         .get_matches();
#     let pattern = cli::pattern_from_os(args.value_of_os("pattern").unwrap())?;
#     let paths = args.values_of_os("path")
#         .map(|osstrs| osstrs.map(|osstr| osstr.to_os_string()).collect())
#         .unwrap_or(vec![OsString::from("./")]);
#     Ok((pattern.to_string(), paths))
# }
{{< /high >}}

If you run this program, then you should see that it has precisely the same
output as our previous example. We weren't able to delete much code, but the
advantage of using the printer here is the number of additional features it
offers with just a simple configuration knob.

For example, to enable colors in your output (that work across platforms,
including in a Windows console), we just need to tweak how the printer and
stdout are built:

{{< high rust "https://github.com/BurntSushi/ripgrep/blob/0.10.0/grep/examples/tutorial-printing-color.rs" >}}
//tutorial-printing-color.rs
# extern crate clap;
# extern crate grep;
# extern crate termcolor;
# extern crate walkdir;
#
# use std::error::Error;
# use std::ffi::OsString;
# use std::io;
# use std::process;
#
# use grep::cli;
use grep::printer::{ColorSpecs, StandardBuilder};
# use grep::regex::RegexMatcher;
# use grep::searcher::Searcher;
# use termcolor::ColorChoice;
# use walkdir::WalkDir;
#
# fn main() {
#     if let Err(err) = try_main() {
#         eprintln!("{}", err);
#         process::exit(1);
#     }
# }

fn try_main() -> Result<(), Box<Error>> {
    let stdout = cli::stdout(
        if cli::is_tty_stdout() {
            ColorChoice::Auto
        } else {
            ColorChoice::Never
        }
    );
    let mut printer = StandardBuilder::new()
        .color_specs(ColorSpecs::default_with_color())
        .build(stdout);

#     let (pattern, paths) = parse_argv()?;
#     let matcher = RegexMatcher::new_line_matcher(&pattern)?;
#     let mut searcher = Searcher::new();
#
#     for path in paths {
#         for result in WalkDir::new(path) {
#             let dir_entry = match result {
#                 Ok(dir_entry) => dir_entry,
#                 Err(err) => {
#                     eprintln!("{}", err);
#                     continue;
#                 }
#             };
#             if !dir_entry.file_type().is_file() {
#                 continue;
#             }
#
#             let path = dir_entry.path();
#             let sink = printer.sink_with_path(&matcher, path);
#             if let Err(err) = searcher.search_path(&matcher, path, sink) {
#                 if err.kind() == io::ErrorKind::BrokenPipe {
#                     return Ok(());
#                 }
#                 eprintln!("{}: {}", path.display(), err);
#                 continue;
#             }
#         }
#     }
#     Ok(())
# }
#
# /// Parse command line arguments into a single pattern and a list of one or
# /// more paths to search.
# fn parse_argv() -> Result<(String, Vec<OsString>), Box<Error>> {
#     use clap::{App, Arg};
#
#     let args = App::new("A simple grep-like example")
#         .version("0.0.1")
#         .arg(Arg::with_name("pattern").required(true))
#         .arg(Arg::with_name("path").multiple(true))
#         .get_matches();
#     let pattern = cli::pattern_from_os(args.value_of_os("pattern").unwrap())?;
#     let paths = args.values_of_os("path")
#         .map(|osstrs| osstrs.map(|osstr| osstr.to_os_string()).collect())
#         .unwrap_or(vec![OsString::from("./")]);
#     Ok((pattern.to_string(), paths))
# }
{{< /high >}}

This example shows the use a few new routines:

* [`is_tty_stdout`](https://docs.rs/grep-cli/*/grep_cli/fn.is_tty_stdout.html)
  provides a cross platform way to determine whether a tty is attached to
  stdout or not (by using the [`atty`](https://docs.rs/atty) library). This
  check ensures that we don't emit ANSI escape sequences when redirecting
  output to a file or another command.
* [`ColorSpecs::default_with_color`](https://docs.rs/grep-printer/*/grep_printer/struct.ColorSpecs.html#method.default_with_color)
  returns a default set of colors, but you can
  [configure your own](https://docs.rs/grep-printer/*/grep_printer/struct.UserColorSpec.html)
  as well.
* [`StandardBuilder`](https://docs.rs/grep-printer/*/grep_printer/struct.StandardBuilder.html)
  can be used to configure a sizable number of formatting options for the
  printer. In this example, we demonstrate how the configuration of which
  colors to use in the printer is distinct from whether we emit colors or not.
  Namely, the color choices are controlled by the printer, but the decision
  of whether to show colors or not is determined by our `stdout` writer.

In the previous section on the `Sink` trait, we showed how to
[emit contextual lines](#code-tutorial-sink-03)
using a custom implementation of `Sink`. We can achieve the same thing here
with our printer:

{{< high rust "https://github.com/BurntSushi/ripgrep/blob/0.10.0/grep/examples/tutorial-printing-context.rs" >}}
//tutorial-printing-context.rs
# extern crate clap;
# extern crate grep;
# extern crate termcolor;
# extern crate walkdir;
#
# use std::error::Error;
# use std::ffi::OsString;
# use std::io;
# use std::process;
#
# use grep::cli;
# use grep::printer::{ColorSpecs, StandardBuilder};
# use grep::regex::RegexMatcher;
use grep::searcher::SearcherBuilder;
# use termcolor::ColorChoice;
# use walkdir::WalkDir;
#
# fn main() {
#     if let Err(err) = try_main() {
#         eprintln!("{}", err);
#         process::exit(1);
#     }
# }

fn try_main() -> Result<(), Box<Error>> {
#    let stdout = cli::stdout(
#        if cli::is_tty_stdout() {
#            ColorChoice::Auto
#        } else {
#            ColorChoice::Never
#        }
#    );
    let mut printer = StandardBuilder::new()
        .color_specs(ColorSpecs::default_with_color())
        .separator_search(Some(b"--".to_vec()))
        .build(stdout);
#
#     let (pattern, paths) = parse_argv()?;
#     let matcher = RegexMatcher::new_line_matcher(&pattern)?;
    let mut searcher = SearcherBuilder::new()
        .before_context(1)
        .after_context(1)
        .build();
#
#     for path in paths {
#         for result in WalkDir::new(path) {
#             let dir_entry = match result {
#                 Ok(dir_entry) => dir_entry,
#                 Err(err) => {
#                     eprintln!("{}", err);
#                     continue;
#                 }
#             };
#             if !dir_entry.file_type().is_file() {
#                 continue;
#             }
#
#             let path = dir_entry.path();
#             let sink = printer.sink_with_path(&matcher, path);
#             if let Err(err) = searcher.search_path(&matcher, path, sink) {
#                 if err.kind() == io::ErrorKind::BrokenPipe {
#                     return Ok(());
#                 }
#                 eprintln!("{}: {}", path.display(), err);
#                 continue;
#             }
#         }
#     }
#     Ok(())
# }
#
# /// Parse command line arguments into a single pattern and a list of one or
# /// more paths to search.
# fn parse_argv() -> Result<(String, Vec<OsString>), Box<Error>> {
#     use clap::{App, Arg};
#
#     let args = App::new("A simple grep-like example")
#         .version("0.0.1")
#         .arg(Arg::with_name("pattern").required(true))
#         .arg(Arg::with_name("path").multiple(true))
#         .get_matches();
#     let pattern = cli::pattern_from_os(args.value_of_os("pattern").unwrap())?;
#     let paths = args.values_of_os("path")
#         .map(|osstrs| osstrs.map(|osstr| osstr.to_os_string()).collect())
#         .unwrap_or(vec![OsString::from("./")]);
#     Ok((pattern.to_string(), paths))
# }
{{< /high >}}

As with our previous example using a custom `Sink` implementation, we need to
ask the searcher to report contextual lines. One other tricky aspect of this
is that we also need to tell the printer to write a separator before printing
the results for a file. It knows not to print the separator before the first
set of results.

Our printer even takes care of executing replacements for us:

{{< high rust "https://github.com/BurntSushi/ripgrep/blob/0.10.0/grep/examples/tutorial-printing-replace.rs" >}}
//tutorial-printing-replace.rs
# extern crate clap;
# extern crate grep;
# extern crate termcolor;
# extern crate walkdir;
#
# use std::error::Error;
# use std::ffi::OsString;
# use std::io;
# use std::process;
#
# use grep::cli;
# use grep::printer::{ColorSpecs, StandardBuilder};
# use grep::regex::RegexMatcher;
# use grep::searcher::Searcher;
# use termcolor::ColorChoice;
# use walkdir::WalkDir;
#
# fn main() {
#     if let Err(err) = try_main() {
#         eprintln!("{}", err);
#         process::exit(1);
#     }
# }

fn try_main() -> Result<(), Box<Error>> {
    let stdout = cli::stdout(
        if cli::is_tty_stdout() {
            ColorChoice::Auto
        } else {
            ColorChoice::Never
        }
    );
    let mut printer = StandardBuilder::new()
        .color_specs(ColorSpecs::default_with_color())
        .replacement(Some(b"foo".to_vec()))
        .build(stdout);

#     let (pattern, paths) = parse_argv()?;
#     let matcher = RegexMatcher::new_line_matcher(&pattern)?;
#     let mut searcher = Searcher::new();
#
#     for path in paths {
#         for result in WalkDir::new(path) {
#             let dir_entry = match result {
#                 Ok(dir_entry) => dir_entry,
#                 Err(err) => {
#                     eprintln!("{}", err);
#                     continue;
#                 }
#             };
#             if !dir_entry.file_type().is_file() {
#                 continue;
#             }
#
#             let path = dir_entry.path();
#             let sink = printer.sink_with_path(&matcher, path);
#             if let Err(err) = searcher.search_path(&matcher, path, sink) {
#                 if err.kind() == io::ErrorKind::BrokenPipe {
#                     return Ok(());
#                 }
#                 eprintln!("{}: {}", path.display(), err);
#                 continue;
#             }
#         }
#     }
#     Ok(())
# }
#
# /// Parse command line arguments into a single pattern and a list of one or
# /// more paths to search.
# fn parse_argv() -> Result<(String, Vec<OsString>), Box<Error>> {
#     use clap::{App, Arg};
#
#     let args = App::new("A simple grep-like example")
#         .version("0.0.1")
#         .arg(Arg::with_name("pattern").required(true))
#         .arg(Arg::with_name("path").multiple(true))
#         .get_matches();
#     let pattern = cli::pattern_from_os(args.value_of_os("pattern").unwrap())?;
#     let paths = args.values_of_os("path")
#         .map(|osstrs| osstrs.map(|osstr| osstr.to_os_string()).collect())
#         .unwrap_or(vec![OsString::from("./")]);
#     Ok((pattern.to_string(), paths))
# }
{{< /high >}}

A real command line program would probably ask the user for the replacement
text, but we can see it working well enough on our Linux checkout:

```
$ simplegrep ATA_PRIV_PM_RESUME
./include/linux/ide.h:51:       foo,    /* resume request */
./include/linux/ide.h:89:                ide_req(rq)->type == foo);
./drivers/ide/ide-pm.c:95:      ide_req(rq)->type = foo;
./drivers/ide/ide-pm.c:251:              ide_req(rq)->type == foo &&
```

Replacements work consistently with colors and support
[capture group interpolation](https://docs.rs/grep-matcher/*/grep_matcher/trait.Captures.html#method.interpolate)
as well.

We're not going to go over every option here, but you can see a list of
documented knobs on
[`StandardBuilder`](https://docs.rs/grep-printer/*/grep_printer/struct.StandardBuilder.html)
in the `grep-printer` library.

The `grep-printer` library also has a
[`JSON` printer](https://docs.rs/grep-printer/*/grep_printer/struct.JSON.html),
which permits printing the matches reported by a `Searcher` in a structured
manner. The format is [JSON Lines](http://jsonlines.org/) and is thoroughly
documented. We even take pains to ensure that we expose data from the
underlying files in a non-lossy way by base64 encoding invalid UTF-8 data.

Using the `JSON` printer is as simple as swapping out the `Standard` printer
in our example:

{{< high rust "https://github.com/BurntSushi/ripgrep/blob/0.10.0/grep/examples/tutorial-printing-json.rs" >}}
//tutorial-printing-json.rs
# extern crate clap;
# extern crate grep;
# extern crate termcolor;
# extern crate walkdir;
#
# use std::error::Error;
# use std::ffi::OsString;
# use std::io;
# use std::process;
#
# use grep::cli;
use grep::printer::JSON;
# use grep::regex::RegexMatcher;
# use grep::searcher::Searcher;
# use termcolor::ColorChoice;
# use walkdir::WalkDir;
#
# fn main() {
#     if let Err(err) = try_main() {
#         eprintln!("{}", err);
#         process::exit(1);
#     }
# }

fn try_main() -> Result<(), Box<Error>> {
    let stdout = cli::stdout(ColorChoice::Never);
    let mut printer = JSON::new(stdout);

#     let (pattern, paths) = parse_argv()?;
#     let matcher = RegexMatcher::new_line_matcher(&pattern)?;
#     let mut searcher = Searcher::new();
#
#     for path in paths {
#         for result in WalkDir::new(path) {
#             let dir_entry = match result {
#                 Ok(dir_entry) => dir_entry,
#                 Err(err) => {
#                     eprintln!("{}", err);
#                     continue;
#                 }
#             };
#             if !dir_entry.file_type().is_file() {
#                 continue;
#             }
#
#             let path = dir_entry.path();
#             let sink = printer.sink_with_path(&matcher, path);
#             if let Err(err) = searcher.search_path(&matcher, path, sink) {
#                 if err.kind() == io::ErrorKind::BrokenPipe {
#                     return Ok(());
#                 }
#                 eprintln!("{}: {}", path.display(), err);
#                 continue;
#             }
#         }
#     }
#     Ok(())
# }

# /// Parse command line arguments into a single pattern and a list of one or
# /// more paths to search.
# fn parse_argv() -> Result<(String, Vec<OsString>), Box<Error>> {
#     use clap::{App, Arg};
#
#     let args = App::new("A simple grep-like example")
#         .version("0.0.1")
#         .arg(Arg::with_name("pattern").required(true))
#         .arg(Arg::with_name("path").multiple(true))
#         .get_matches();
#     let pattern = cli::pattern_from_os(args.value_of_os("pattern").unwrap())?;
#     let paths = args.values_of_os("path")
#         .map(|osstrs| osstrs.map(|osstr| osstr.to_os_string()).collect())
#         .unwrap_or(vec![OsString::from("./")]);
#     Ok((pattern.to_string(), paths))
# }
{{< /high >}}

You should expect the JSON printer to be quite fast. Thanks to
[serde_json](https://docs.rs/serde_json),
we can print each result without any allocation whatsoever (in the common
case).
