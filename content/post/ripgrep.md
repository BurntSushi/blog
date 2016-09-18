+++
date = "2016-09-17T14:39:00-04:00"
title = "ripgrep is faster than {grep, ag, git grep, ucg, pt, sift}"
author = "Andrew Gallant"
url = "ripgrep"

[blackfriday]
plainIdAnchors = true
+++

In this article I will introduce a new command line search tool, `ripgrep`,
that combines the usability of
[The Silver Searcher](https://github.com/ggreer/the_silver_searcher)
(an [`ack`](http://beyondgrep.com/) clone) with the
raw performance of GNU grep. `ripgrep` is fast, cross platform (with binaries
available for Linux, Mac and Windows) and written in
[Rust](https://www.rust-lang.org).

We will attempt to do the impossible: a fair benchmark comparison between
several popular code search tools. Specifically, we will dive into a series of
25 benchmarks that substantiate the following claims:

* For both searching single files *and* huge directories of files, no other
  tool obviously stands above `ripgrep` in either performance or correctness.
* `ripgrep` is the only tool with proper Unicode support that doesn't make
  you pay dearly for it.
* Tools that search many files at once are generally *slower* if they use
  memory maps, not faster.

As the author of both `ripgrep` and
[the underlying regular expression engine](https://github.com/rust-lang-nursery/regex),
I will use this opportunity to provide detailed insights into the performance
of each code search tool. No benchmark will go unscrutinized!

**Target audience**: Some familiarity with programming and some experience with
working on the command line.

<!--more-->

## Screenshot of search results

[![A screenshot of a sample search with ripgrep](http://burntsushi.net/stuff/ripgrep1.png)](http://burntsushi.net/stuff/ripgrep1.png)

## Table of Contents


This article is split into four sections. The first briefly introduces
`ripgrep`. The second discusses benchmarking methodology. The third section
presents benchmarks against the repository of the Linux kernel. The fourth
section presents benchmarks against single files.

If you came for the comparison between `ripgrep` and The Silver Searcher, then
the third section is for you. If you came for the comparison between `ripgrep`
and GNU Grep, then the fourth section is for you. We will, of course, compare
several other tools as well.

* [Introducing ripgrep](#introducing-ripgrep)
    * [Pitch](#pitch)
    * [Anti-pitch](#anti-pitch)
    * [Installation](#installation)
    * [Whirlwind tour](#whirlwind-tour)
* [Methodology](#methodology)
    * [Overview](#overview)
    * [Benchmark runner](#benchmark-runner)
    * [Environment](#environment)
* [Code search benchmarks](#code-search-benchmarks)
    * [`linux_literal_default`](#linux-literal-default)
    * [`linux_literal`](#linux-literal)
    * [`linux_literal_casei`](#linux-literal-casei)
    * [`linux_word`](#linux-word)
    * [`linux_unicode_word`](#linux-unicode-word)
    * [`linux_re_literal_suffix`](#linux-re-literal-suffix)
    * [`linux_alternates`](#linux-alternates)
    * [`linux_alternates_casei`](#linux-alternates-casei)
    * [`linux_unicode_greek`](#linux-unicode-greek)
    * [`linux_unicode_greek_casei`](#linux-unicode-greek-casei)
    * [`linux_no_literal`](#linux-no-literal)
* [Single file benchmarks](#single-file-benchmarks)


## Introducing ripgrep

### Pitch

Why should you use `ripgrep` over any other search tool? Well...

* It can replace both The Silver Searcher and GNU grep because it is faster
  than both. (N.B. It is not, strictly speaking, an interface compatible
  "drop-in" replacement for both, but the feature sets are far more similar
  than different.)
* Like The Silver Searcher, `ripgrep` defaults to recursive directory search
  and won't search files ignored by your `.gitignore` files. It also ignores
  hidden and binary files by default. `ripgrep` also implements full support
  for `.gitignore`, where as there are many bugs related to that functionality
  in The Silver Searcher. Of the things that The Silver Searcher gets wrong,
  `ripgrep`  supports `.gitignore` priority (including in parent directories
  and sub-directories), whitelisting and recursive globs.
* `ripgrep` can search specific types files. For example, `rg -tpy foo` limits
  your search to Python files and `rg -Tjs foo` excludes Javascript files
  from your search. `ripgrep` can be taught about new file types with custom
  matching rules.
* `ripgrep` supports many features found in `grep`, such as showing the context
  of search results, searching multiple patterns, highlighting matches with
  color and full Unicode support. Unlike GNU grep, `ripgrep` stays fast while
  supporting Unicode (which is always on).

In other words, use `ripgrep` if you like speed, fewer bugs and Unicode.

### Anti-pitch

I'd like to try to convince you why you *shouldn't* use `ripgrep`. Often, this
is far more revealing than reasons why I think you *should* use `ripgrep`.

* `ripgrep` uses a regex engine based on finite automata, so if you want fancy
  regex features such as backreferences or look around, `ripgrep` won't give
  them to you. `ripgrep` does support lots of things though, including, but not
  limited to: lazy quantification (e.g., `a+?`), repetitions (e.g., `a{2,5}`),
  begin/end assertions (e.g., `^\w+$`), word boundaries (e.g., `\bfoo\b`), and
  support for Unicode categories (e.g., `\p{Sc}` to match currency symbols or
  `\p{Lu}` to match any uppercase letter).
* If you need to search files with text encodings other than UTF-8 (like
  UTF-16), then `ripgrep` won't work. `ripgrep` will still work on ASCII
  compatible encodings like latin1 or otherwise partially valid UTF-8.
  `ripgrep` may grow support for additional text encodings over time.
* `ripgrep` doesn't yet support searching for patterns/literals from a file,
  but this is easy to add and should change soon.
* If you need to search compressed files. `ripgrep` doesn't try to do any
  decompression before searching.

In other words, if you like fancy regexes, non-UTF-8 character encodings or
decompressing and searching on-the-fly, then `ripgrep` may not quite meet your
needs (yet).

### Installation

The binary name for `ripgrep` is `rg`.

[Binaries for `ripgrep` are available for Windows, Mac and
Linux.](https://github.com/BurntSushi/ripgrep/releases) Linux binaries are
static executables. Windows binaries are available either as built with MinGW
(GNU) or with Microsoft Visual C++ (MSVC). When possible, prefer MSVC over GNU,
but you'll need to have the
[Microsoft Visual C++ Build
Tools](http://landinghub.visualstudio.com/visual-cpp-build-tools)
installed.

If you're a Rust programmer, `ripgrep` can be installed with `cargo`:

{{< high sh >}}
$ cargo install ripgrep
{{< /high >}}

If you'd like to build `ripgrep` from source, that is also easy to do.
`ripgrep` is written in Rust, so you'll need to grab a
[Rust installation](https://www.rust-lang.org/) in order to compile it.
`ripgrep` compiles with Rust 1.9 (stable) or newer. To build:

{{< high sh >}}
$ git clone git://github.com/BurntSushi/ripgrep
$ cd ripgrep
$ cargo build --release
$ ./target/release/rg --version
0.1.2
{{< /high >}}

If you have a Rust nightly compiler, then you can enable optional SIMD
acceleration like so:

{{< high sh >}}
RUSTFLAGS="-C target-cpu=native" cargo build --release --features simd-accel
{{< /high >}}

N.B. `ripgrep` is not yet available in any package repositories. I'd like to
fix that in the future.

### Whirlwind tour

The command line usage of `ripgrep` doesn't differ much from other tools that
perform a similar function, so you probably already know how to use `ripgrep`.
The full details can be found in `rg --help`, but let's go on a whirlwind tour.

`ripgrep` detects when its printing to a terminal, and will automatically
colorize your output and show line numbers, just like The Silver Searcher.
Coloring works on Windows too! Colors can be controlled more granularly with
the `--color` flag.

One last thing before we get started: `ripgrep` assumes UTF-8 *everywhere*. It
can still search files that are invalid UTF-8 (like, say, latin-1), but it will
simply not work on UTF-16 encoded files or other more exotic encodings.
[Support for other encodings may
happen.](https://github.com/BurntSushi/ripgrep/issues/1).

To recursively search the current directory, while respecting all `.gitignore`
files:

{{< high sh >}}
$ rg foobar
{{< /high >}}

The above command also respects all `.rgignore` files, including in parent
directories. `.rgignore` files can be used when `.gitignore` files are
insufficient. In all cases, `.rgignore` patterns take precedence over
`.gitignore`.

To ignore all ignore files, use `--no-ignore`:

{{< high sh >}}
$ rg --no-ignore foobar
{{< /high >}}

(Tip: If your ignore files aren't being adhered to like you expect, run your
search with the `--debug` flag.)

Make the search case insensitive with `-i`, invert the search with `-v` or
show the 2 lines before and after every search result with `-C2`.

Force all matches to be surrounded by word boundaries with `-w`.

Search and replace (find first and last names and swap them):

{{< high sh >}}
$ rg '([A-Z][a-z]+)\s+([A-Z][a-z]+)' --replace '$2, $1'
{{< /high >}}

Named groups are supported:

{{< high sh >}}
$ rg '(?P<first>[A-Z][a-z]+)\s+(?P<last>[A-Z][a-z]+)' --replace '$last, $first'
{{< /high >}}

Up the ante with full Unicode support, by matching any uppercase Unicode letter
followed by any sequence of lowercase Unicode letters (good luck doing this
with other search tools!):

{{< high sh >}}
$ rg '(\p{Lu}\p{Ll}+)\s+(\p{Lu}\p{Ll}+)' --replace '$2, $1'
{{< /high >}}

Search only HTML and CSS files:

{{< high sh >}}
$ rg -thtml -tcss foobar
{{< /high >}}

Search everything except for Javascript files:

{{< high sh >}}
$ rg -Tjs foobar
{{< /high >}}

To see a list of types supported, run `rg --type-list`. To add a new type, use
`--type-add`:

{{< high sh >}}
$ rg --type-add 'foo:*.foo,*.foobar'
{{< /high >}}

The type `foo` will now match any file ending with the `.foo` or `.foobar`
extensions.

## Methodology

### Overview

Coming up with a good and fair benchmark is *hard*, and I have assuredly made
some mistakes in doing so. In particular, there are so many variables to
control for that testing every possible permutation isn't feasible. This means
that the benchmarks I'm presenting here are *curated*, and, given that I am the
author of one of the tools in the benchmark, they are therefore also *biased*.
Nevertheless, even if I fail in my effort to provide a fair benchmark suite, I
do hope that some of you may find my analysis interesting, which will try to
explain the results in each benchmark.

In other words, I'm pretty confident that I've gotten the *details* correct,
but I could have missed something in the bigger picture. Because of that, let's
go over some important insights that guided construction of this benchmark.

* Focus on the problem that an *end user* is trying to solve. For example, we
  split the entire benchmark in two: one for searching a large directory of
  files and one for searching a single large file. The former might correspond
  to an end user searching their code while the latter might correspond to an
  end user searching logs. As we will see, these two use cases have markedly
  different performance characteristics. A tool that is good at one isn't
  necessarily good at the other. (The premise of `ripgrep` is that it is
  possible to be good at both!)
* Apply *end user* problems more granularly as well. For example, most
  searches result in few hits relative to the corpus search, so prefer
  benchmarks that report few matches. Another example: I hypothesize, based on
  my own experience, that most searches use patterns that are simple literals,
  alternations or very light regexes, so bias the benchmarks towards those
  types of patterns.
* Almost every search tool has slightly different default behavior, and these
  behavioral changes can have an impact on performance. There is some value in
  looking at "out-of-the-box" performance, and we therefore do look at a
  benchmark for that, but stopping there is a bit unsatisfying. If our goal is
  to do a *fair* comparison, then we need to at least try to convince each tool
  to do roughly the same work, **from the perspective of an end user**. A good
  example of this is reporting line numbers. Some tools don't provide a way of
  disabling line counting, so when doing comparisons between tools that do, we
  need to explicitly enable line numbers. This is important, because counting
  lines can be quite costly! A good *non-example* of this is if one tool uses
  memory maps and another uses an intermediate buffer. This is an
  implementation choice, and not one that alters what the user actually sees,
  therefore comparing those two implementation choices in a benchmark is
  completely fair (assuming an analysis that points it out).

With that out of the way, let's get into the nitty gritty. First and foremost,
what tools are we benchmarking?

* [`ripgrep` (rg)](https://github.com/BurntSushi/ripgrep) (v0.1.2) - You've
  heard enough about this one already.
* [GNU grep](https://www.gnu.org/software/grep/) (v2.25) - Ol' reliable.
* [git grep](https://www.kernel.org/pub/software/scm/git/docs/git-grep.html)
  (v2.7.4) -
  Like `grep`, but built into `git`. Only works well in `git` repositories.
* [The Silver Searcher (ag)](https://github.com/ggreer/the_silver_searcher)
  (commit `cda635`, using PCRE 8.38) - Like `ack`, but written in C and much
  faster. Reads your `.gitignore` files just like `ripgrep`.
* [Universal Code Grep (ucg)](https://github.com/gvansickle/ucg) (commit
  `487bfb`, using PCRE 10.21) - Also like `ack` but written in C++, and only
  searches files from a whitelist, and doesn't support reading `.gitignore`.
* [The Platinum Searcher
(pt)](https://github.com/monochromegane/the_platinum_searcher) (commit
  `509368`) - Written in Go and does support `.gitignore` files.
* [sift](https://github.com/svent/sift) (commit `2d175c`) - Written in Go and
  supports `.gitignore` files with an optional flag, but generally prefers
  searching everything (unlike every other tool in this list except for
  `grep`).

Notably absent from this list is `ack`. We don't benchmark it here because it
is outrageously slow. Even on the simplest benchmark (a literal in the Linux
kernel repository), `ack` is around **two** orders of magnitude slower than
`ripgrep`. It's just not worth it.

### Benchmark runner

The benchmark runner is a Python program (requires at least Python 3.5) that
you can use to not only run the benchmarks themselves, but download the corpora
used in the benchmarks as well. The script is called `benchsuite` and
[is in the `ripgrep`
repository](https://github.com/BurntSushi/ripgrep/blob/master/benchsuite/benchsuite).
You can use it like so:

{{< high sh >}}
$ git clone git://github.com/BurntSushi/ripgrep
$ cd ripgrep/benchsuite
# WARNING! This downloads several GB of data, and builds the Linux kernel.
# This took about 15 minutes on a high speed connection.
# Tip: try `--download subtitles-ru` to grab the smallest corpus, but you'll
# be limited to running benchmarks for only that corpus.
$ ./benchsuite --dir /path/to/data/dir --download all
# List benchmarks available.
$ ./benchsuite --dir /path/to/data/dir --list
# Run a benchmark.
# Omit the benchmark name to run all benchmarks. The full suite can take around
# 30 minutes to complete.
$ ./benchsuite --dir /path/to/data/dir '^subtitles_ru_literal$'
{{< /high >}}

If you don't have all of the code search tools used in the benchmarks, then
pass `--allow-missing` to give `benchsuite` permission to skip running them. To
save the raw data (the timing for every command run), pass `--raw
/path/to/raw.csv`.

The benchmark runner tries to do a few basic things for us to help reduce the
chance that we get misleading data:

* Every benchmarked command is run once before being measured as a "warm up."
  Specifically, this is to ensure that the corpora being searched is already in
  the operating system's page cache. If we didn't do this, we might end up
  benchmarking disk I/O, which is not only uninteresting for our purposes, but
  is probably not a common end user scenario. It's more likely that you'll be
  executing lots of searches against the same corpus (at least, I know I do).
* Every benchmarked command is run three times, with a timing recorded for each
  run. The final "result" of that command is its distribution (mean +/-
  standard deviation). If I were a statistician, I could probably prove that
  three samples is insufficient. Nevertheless, getting more samples takes more
  time, and for the most part, the variance is very low from run to run.

The benchmark definitions themselves are responsible for making sure each
command is trying to do similar work as other commands we're comparing it to.
For example, we need to be careful to enable and disable Unicode support in GNU
grep where appropriate, because full Unicode handling can make GNU grep run
very slowly. Within each benchmark, there are often multiple variables of
interest. To account for this, I've added labels like `(ASCII)` or
`(whitelist)` where appropriate. We'll dig into those labels in more detail
later.

Please also feel encouraged to add your own benchmarks if you'd like to play
around. The benchmarks are in the top-half of the file, and it should be fairly
straight-forward to copy & paste another benchmark and modify it. Simply
defining a new benchmark will make it available. The second half of the script
is the runner itself and probably shouldn't need to be modified.

### Environment

The actual environment used to run the benchmarks presented in this article was
a `c3.2xlarge` instance on Amazon EC2. It ran Ubuntu 16.04, had a Xeon E5-2680
2.8 GHz CPU, 16 GB of memory and an 80 GB SSD (on which the corpora was
stored). This was enough memory to fit all of the corpora in memory. The box
was specifically provisioned for the purpose of running benchmarks, so it was
not doing anything else.

The full log of system setup and commands I used to install each of the search
tools and run benchmarks can be found
[here](https://github.com/BurntSushi/ripgrep/blob/master/benchsuite/runs/2016-09-17-ubuntu1604-ec2/README.SETUP).
I also captured the
[output of the bench runner (SPOILER ALERT)](https://github.com/BurntSushi/ripgrep/blob/master/benchsuite/runs/2016-09-17-ubuntu1604-ec2/summary)
and the
[raw output](https://github.com/BurntSushi/ripgrep/blob/master/benchsuite/runs/2016-09-17-ubuntu1604-ec2/raw.csv),
which includes the timings, full set of command line arguments and any
environment variables set for every command run in every benchmark.

## Code search benchmarks

This is the first half of our benchmarks, and corresponds to an *end user*
trying to search a large repository of code for a particular pattern.

The corpus used for this benchmark is a *built* checkout of the Linux kernel,
specifically commit `d0acc7`. We actually build the Linux kernel because the
process of building the kernel leaves a lot of garbage in the repository that
you probably don't want to search. This can influence not only the relevance of
the results returned by a search tool, but the performance as well.

All benchmarks run in this section were run in the root of the repository.
Remember, you can see the full
[raw results of each command](https://github.com/BurntSushi/ripgrep/blob/master/benchsuite/runs/2016-09-17-ubuntu1604-ec2/raw.csv)
if you like. The benchmark names correspond to the headings below.

Without further ado, let's start looking at benchmarks.

### `linux_literal_default`

**Pattern**: `PM_RESUME`

**Description**: This benchmark compares the time it takes to execute a simple
literal search using each tool's default settings. This is an intentionally
unfair benchmark meant to highlight the differences between tools and their
"out-of-the-box" settings.

{{< high text >}}
rg         0.277 +/- 0.002 (lines: 16)
ag         1.594 +/- 0.008 (lines: 16)
ucg        0.219 +/- 0.007 (lines: 16)*+
pt         0.439 +/- 0.024 (lines: 16)
sift       0.344 +/- 0.014 (lines: 16)
git grep   0.344 +/- 0.005 (lines: 16)
{{< /high >}}

* `*` - Best mean time.
* `+` - Best sample time.
* `rg == ripgrep`, `ag == The Silver Searcher`, `ucg == Universal Code Grep`,
  `pt == The Platinum Searcher`

**Analysis**: We'll first start by actually describing what each tool is doing:

* `rg` respects the Linux repo's `.gitignore` files (of which there are
  `178`(!) of them), and skips hidden and binary files. `rg` does not count
  lines.
* `ag` has the same default behavior as `rg`, except it counts lines.
* `ucg` also counts lines, but does not attempt to read `.gitignore` files.
  Instead, it only searches files from an (extensible) whitelist according
  to a set of glob rules. For example, both `rg` and `ag` will search
  `fs/jffs2/README.Locking` while `ucg` won't, because it doesn't recognize
  the `Locking` extension. (A search tool probably *should* search that file,
  although it does not impact the results of this specific benchmark.)
* `pt` has the same default behavior as `ag`.
* `sift` searches everything, including binary files and hidden files. It
  *should* be equivalent to `grep -r`, for example. It also does not count
  lines.
* `git grep` should have the same behavior at `rg`, and similarly does not
  count lines.

The high-order bit to extract from this benchmark is that a naive comparison
between search tools is completely unfair from the perspective of performance,
but is really important if you care about the *relevance* of results returned
to you. `sift`, like `grep -r`, will throw everything it can back at you, which
is totally at odds with the philosophy behind every other tool in this
benchmark: only return results that are *probably* relevant. Things inside your
`.git` probably aren't, for example. (This isn't to say that `sift`'s
philosophy is wrong. The tool is clearly intended to be configured by an end
user to their own tastes, which has its own pros and cons.)

With respect to performance, there are two key variables to pay attention to.
They will appear again and again throughout our benchmark:

* Counting lines *can be* quite expensive. A naive solution---a loop over every
  byte and comparing it to a `\n`---will be quite slow for example.
  [Universal Code Grep counts lines using SIMD](https://github.com/gvansickle/ucg/blob/8bbebc002bbf112d147928f89677cba703d007bb/src/FileScanner_sse4_2.cpp#L190)
  and
  [`ripgrep` counts lines using packed comparisons (16 bytes at a time)](https://github.com/BurntSushi/ripgrep/blob/919c5c72994edb378706594f6268542983eeee6d/src/search_stream.rs#L549).
  However, in the Linux code search benchmarks, because the size of each
  individual file is very small and the number of matches is tiny compared
  to the corpus size, the time spent counting lines tends to not be so
  significant. Especially since every tool in this benchmark parallelizes
  search to some degree. When we get to the single-file benchmarks, this
  variable will become much more pertinent.
* Respecting `.gitignore` files incurs some amount of overhead. Even though
  respecting `.gitignore` reduces the number of files searched, it can be
  slower overall to actually read the patterns, compile them and match them
  against every path than to just search every file. This is precisely how
  `ucg` soundly beats `ripgrep` in this benchmark. (We will control for this
  variable in future benchmarks.) In other words, respecting `.gitignore` is a
  feature that improves *relevance* first and foremost. It is strictly a bonus
  if it also happens to improve performance.

The specific reasons why supporting `.gitignore` leads to a slower overall
search are:

* Every directory descended requires looking for a corresponding `.gitignore`.
  Multiply the number of calls if you support additional ignore files, like
  both The Silver Searcher and `ripgrep` do. The Linux kernel repository has
  `4,640` directories. `178` of them have `.gitignore` files.
* Each `.gitignore` file needs to be compiled into something that can match
  file paths. Both The Silver Searcher and `ripgrep` use tricks to make this
  faster. For example, simple patterns like `/vmlinux` or `*.o` can be matched
  using simple literal comparisons or by looking at the file extension of a
  candidate path and comparing it directly. For more complex patterns like
  `*.c.[012]*.*`, a full glob matcher needs to be used. The Silver Searcher
  uses `fnmatch` while `ripgrep` translates all such globs into a single
  regular expression which can be matched against a single path all at once.
  Doing all this work takes time.
* Actually matching a path has non-trivial overhead that must be paid for
  *every* path searched. The compilation phase described above is complex
  precisely for making this part faster. We try to stay out of the regex
  machinery as best we can, but we can't avoid it completely.

In contrast, a whitelist like the one used by `ucg` is comparatively easy to
make fast. The set of globs is known upfront, so no additional checks need to
be made while traversing the file tree. Moreover, the globs tend to be of the
`*.ext` variety, which fall into the bucket of globs that can be matched
efficiently just by looking at the extension of a file path.

The downside of a whitelist is obvious: you might end up search results simply
because `ucg` didn't know about a particular file extension.

### `linux_literal`

**Pattern**: `PM_RESUME`

**Description**: This benchmark runs the same query as in the
`linux_literal_default` benchmark, but we try to do a fair comparison. In
particular, we run `ripgrep` in two modes: one where it respects `.gitignore`
files (corresponding to the `(ignore)` label) and one where it uses a whitelist
and doesn't respect `.gitignore` (corresponding to the `(whitelist)` label).
The former mode is comparable to `ag`, `pt`, `sift` and `git grep`, while the
latter mode is comparable to `ucg`. We also run `rg` a third time by
explicitly telling it to use memory maps for search, which matches the
implementation strategy used by `ag`. `sift` is run such that it respects
`.gitignore` files and excludes binary, hidden and PDF files. All commands
executed here count lines, because some commands (`ag` and `ucg`) don't support
disabling line counting.

{{< high text >}}
rg (ignore)          0.348 +/- 0.054 (lines: 16)
rg (ignore) (mmap)   1.597 +/- 0.013 (lines: 16)
ag (ignore) (mmap)   1.590 +/- 0.003 (lines: 16)
pt (ignore)          0.455 +/- 0.015 (lines: 16)
sift (ignore)        0.630 +/- 0.001 (lines: 16)
git grep (ignore)    0.344 +/- 0.004 (lines: 16)
rg (whitelist)       0.253 +/- 0.077 (lines: 16)+
ucg (whitelist)      0.222 +/- 0.005 (lines: 16)*
{{< /high >}}

* `*` - Best mean time.
* `+` - Best sample time.

**Analysis**: We have a ton of ground to cover on this one.

First and foremost, the `(ignore)` vs. `(whitelist)` variables have a clear
impact on the performance of `rg`. We won't rehash all the details from the
analysis in `linux_literal_default`, but switching `rg` into its whitelist mode
brings it into a dead heat with `ucg`.

Secondly, `ucg` is just as fast as `ripgrep`, even though I've said that
`ripgrep` is the fastest. It turns out that `ucg` and `rg` are pretty evenly
matched when searching for plain literals in large repositories. We will see a
stronger separation in later benchmarks. Still, what makes `ucg` fast?

* Like `ripgrep`, `ucg` does searching with an intermediate buffer.
* It has a fast explicitly SIMD based line counting algorithm. `ripgrep` has
  something similar, but relies on the compiler for autovectorization.
* `ucg` uses PCRE2's JIT, which is *insanely* fast. In my own very rough
  benchmarks, PCRE2's JIT is one of the few general purpose regex engines that
  is competitive with Rust's regex engine (on regexes that don't expose PCRE's
  exponential behavior due to backtracking, since Rust's regex engine doesn't
  suffer from that weakness).
* `ucg` parallelizes directory traversal, which is something that `ripgrep`
  doesn't do. `ucg` has it a bit easier here because it doesn't support
  `.gitignore` files. Parallelizing directory traversal while maintaining state
  for `.gitignore` files in a way that scales isn't a problem I've figured out
  how to cleanly solve yet.

Both `sift` and `pt` perform almost as well as `ripgrep`. In fact, both `sift`
and `pt` do implement a parallel recursive directory traversal while
still respecting `.gitignore` files, which is likely one reason for their
speed. As we will see in future benchmarks, their speed here is misleading.
Namely, they are fast because they stay outside of Go's regexp engine by virtue
of being a literal. (There will be more discussion on this point later.)

Finally, what's going on with The Silver Searcher? Is it really that much
slower than everything else? The key here is that its use of memory maps is
making it *slower*, not faster (in direct contradiction to the claims in its
README).

OK, let's pause and pop up a level to talk about what this actually means.
First, we need to consider how these search tools fundamentally work. Generally
speaking, a search tool like this has two ways of actually searching files on
disk:

1. It can memory map the file and search the entire file all at once *as if* it
   were a single contiguous region of bytes in memory. The operating system
   does the work behind the scenes to make a file look like a contiguous
   region of memory. This particular approach is *really* convenient when
   comparing it to the alternative described next.
2. ... or it can allocate an intermediate buffer, read a fixed size block of
   bytes from the file into it, search the buffer and then repeat the process.
   This particular approach is absolutely ghoulish to implement, because you
   need to account for the fact that a buffer may end in the middle of the
   line. You also need to account for the fact that a single line may exceed
   the size of your buffer. Finally, if you're going to support showing the
   lines around a match (its "context") as both `grep` and `ripgrep` do, then
   you need to do additional bookkeeping to make sure any lines from a previous
   buffer are printed even if a match occurs at the beginning of the next block
   read from the file.

Naively, it seems like (1) would be *obviously* faster. Surely, all of the
bookkeeping and copying in (2) would make it much slower! In fact, this is not
at all true. (1) may not require much bookkeeping from the perspective of the
programmer, but there is a lot of [bookkeeping going on inside the
Linux kernel to maintain the memory
map](http://lkml.iu.edu/hypermail/linux/kernel/0004.0/0728.html). (That link
goes to a mailing list post that is quite old, but it still appears relevant
today.)

When I first started writing `ripgrep`, I used the memory map approach. It took
me a long time to be convinced enough to start down the second path with an
intermediate buffer (because neither a CPU profile nor the output of `strace`
ever showed any convincing evidence that memory maps were to blame), but as
soon as I had a prototype of (2) working, it was clear that it was much faster
than the memory map approach.

With all that said, memory maps aren't all bad. They just happen to be bad for
the particular use case of "rapidly open, scan and close memory maps for
thousands of small files." For a different use case, like, say, "open this
large file and search it once," memory maps turn out to be a boon. We'll see
that in action in our single-file benchmarks later.

The key datapoint that supports this conclusion is the comparison between
`rg (ignore)` and `rg (ignore) (mmap)`. In particular, this controls for
everything *except* for the search strategy and fairly conclusively points
right at memory maps as the problem.

With all that said, the performance of memory maps is very dependent on your
environment, and the absolute difference between `rg (ignore)` and `ag (ignore)
(mmap)` can be misleading. In particular, since these benchmarks were run on an
EC2 `c3.2xlarge`, we were probably inside a virtual machine, which could
feasibly impact memory map performance. To test this, I ran the same benchmark
on my machine under my desk (Intel i7-6900K 3.2 GHz, 16 CPUs, 64 GB memory,
SSD) and got these results:

{{< high text >}}
rg (ignore)          0.156 +/- 0.006 (lines: 16)
rg (ignore) (mmap)   0.397 +/- 0.013 (lines: 16)
ag (ignore) (mmap)   0.444 +/- 0.016 (lines: 16)
pt (ignore)          0.159 +/- 0.008 (lines: 16)
sift (ignore)        0.344 +/- 0.002 (lines: 16)
git grep (ignore)    0.195 +/- 0.023 (lines: 16)
rg (whitelist)       0.108 +/- 0.005 (lines: 16)*+
ucg (whitelist)      0.165 +/- 0.005 (lines: 16)
{{< /high >}}

<!--*-->

`rg (ignore)` still soundly beats `ag`, and our memory map conclusions above
are still supported by this data, but the difference between `rg (ignore)` and
`ag (ignore) (mmap)` has narrowed quite a bit!

### `linux_literal_casei`

**Pattern**: `PM_RESUME`

**Description**: This benchmark is like `linux_literal`, except it asks the
search tool to perform a case insensitive search.

{{< high text >}}
rg (ignore)          0.423 +/- 0.118 (lines: 370)
rg (ignore) (mmap)   1.615 +/- 0.010 (lines: 370)
ag (ignore) (mmap)   1.601 +/- 0.027 (lines: 370)
sift (ignore)        0.804 +/- 0.003 (lines: 370)
git grep (ignore)    0.346 +/- 0.004 (lines: 370)
rg (whitelist)       0.234 +/- 0.032 (lines: 370)
ucg (whitelist)      0.220 +/- 0.008 (lines: 370)*+
{{< /high >}}

* `*` - Best mean time.
* `+` - Best sample time.
* `pt` was removed from this benchmark because it is over 10 times slower than
  the next slowest tool.

**Analysis**: The biggest change from the previous benchmark is that `pt` got
an order of magnitude slower than the next slowest tool, and was therefore
dropped.

(N.B. I use "drop tools than are an order of magnitude slower than the next
slowest tool" as a rule of thumb throughout the rest of the benchmarks.
It's *mostly* for practical purposes. Some tools take so long on some queries
that the total benchmark time would balloon. We will still address *why* they
got dropped, though.)

So why did `pt` get so slow? In particular, both `sift` and `pt` use Go's
`regexp` package for searching, so why did one perish while the other only got
slightly slower? It turns out that when `pt` sees the `-i` flag indicating case
insensitive search, it will force itself to use Go's `regexp` engine with the
`i` flag set. So for example, given a CLI invocation of `pt -i foo`, it will
translate that to a Go regexp of `(?i)foo`, which will handle the case
insensitive search.

On the other hand, `sift` will notice the `-i` flag and take a different route.
`sift` will lowercase both the pattern and every block of bytes it searches.
(`sift`, like the rest of the tools sans `ag`, does searching with an
intermediate buffer.) This filter over all the bytes searched is likely the
cause of `sift`'s performance drop from the previous `linux_literal` benchmark.
(It's worth pointing out that this optimization is actually incorrect, because
it only accounts for ASCII case insensitivity, and not full Unicode case
insensitivity, which `pt` gets by virture of Go's regexp engine.)

But still, is Go's regexp engine really that slow? Unfortunately, yes, it is.
While Go's regexp engine takes worst case linear time on all searches (and is
therefore exponentially faster than even PCRE2 for some set of regexes and
corpora), its actual implementation hasn't quite matured yet. Indeed, every
*fast* regex engine based on finite automata (like Go's regexp engine) that I'm
aware of implements some kind of DFA engine. For example, GNU grep, Google's
RE2 and Rust's regex library all do this. Go's does not (but there is work in
progress to make this happen, so perhaps `pt` will get faster on this benchmark
without having to do anything at all!).

There is one other thing worth noting here before moving on. Namely, that `rg`,
`ag`, `git grep` and `ucg` didn't noticeably change much from the previous
benchmark. Shouldn't a case insensitive search incur some kind of overhead? The
answer is complicated and actually requires more knowledge of the underlying
regex engines than I have. Thankfully, I can at least answer it for Rust's
regex engine.

The key insight is that a case insensitive search for `PM_RESUME` is precisely
the same as a case sensitive search of the alternation of all possible case
agnostic versions of `PM_RESUME`. So for example, it might start like:
`PM_RESUME|pM_RESUME|Pm_RESUME|PM_rESUME|...` and so on. Of course, the
full alternation, even for a small literal like this, would be *quite* large.
The key is that we can extract a small prefix and enumerate all of *its*
combinations quite easily. In this case, Rust's regex engine figures out this
alternation (which you can see by passing `--debug` to `rg` and examining
`stderr`):

{{< high text >}}
PM_RE
PM_Re
PM_rE
PM_re
Pm_RE
Pm_Re
Pm_rE
Pm_re
pM_RE
pM_Re
pM_rE
pM_re
pm_RE
pm_Re
pm_rE
pm_re
{{< /high >}}

(Rest assured that Unicode support is baked into this process. For example, a
case insensitive search for `S` would yield the following literals: `S`, `s`
and `ſ`.)

Now that we have this alternation of literals, what do we do with them? The
classical answer is to compile them into a DFA
(perhaps [Aho-Corasick](https://github.com/BurntSushi/aho-corasick)),
and use it as a way to quickly skip through the search text. A match of any of
the literals would then cause the regex engine to activate and try to verify
the match. This way, we aren't actually running the entire search text through
the regex engine, which could be quite a bit slower.

But, Rust's regex engine doesn't actually use Aho-Corasick for this. When SIMD
acceleration is enabled (and you can be sure it is for these benchmarks, and
for the binaries I distribute), a special multiple pattern search algorithm
called Teddy is used. The algorithm is unpublished, but was invented by
Geoffrey Langdale as part of [Intel's Hyperscan regex
library](https://github.com/01org/hyperscan). The algorithm works roughly by
using packed comparisons of 16 bytes at a time to find candidate locations
where a literal might match.
[I adapted the algorithm from the Hyperscan project to
Rust](https://github.com/rust-lang-nursery/regex/blob/master/src/simd_accel/teddy128.rs),
and included an extensive write up in the comments if you're interested.

While Teddy doesn't buy us much over other tools in this particular benchmark,
we will see much larger wins in later benchmarks.


### `linux_word`

### `linux_unicode_word`

### `linux_re_literal_suffix`

### `linux_alternates`

### `linux_alternates_casei`

### `linux_unicode_greek`

### `linux_unicode_greek_casei`

### `linux_no_literal`

## Single file benchmarks
