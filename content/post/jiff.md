+++
date = "2026-11-09T16:55:00-05:00"
title = "A datetime library for Rust"
url = "jiff"

[blackfriday]
plainIdAnchors = true
+++

[Jiff] is a datetime library for Rust that encourages you to jump into the
[pit of success]. Jiff automatically handles many common problems related to
daylight saving time by putting time zone handling front and center into the
API. Jiff has already seen widespread [adoption] in the Rust ecosystem.

Jiff is pronounced like "gif" with a soft "g," as in "gem."

This blog announces the Jiff 1.0 release by giving a tour of its API.

**Target audience**: Rust programmers who need convenient datetime handling.

<!--more-->

## Table of Contents

* [Introduction](#introduction)
* [The DateTime Onion](#the-datetime-onion)
* [Motivation](#motivation)
* [Inspiration](#inspiration)

## Introduction

dates, times, models, being wrong?

## The DateTime Onion

I've come to think of the datetime types in Jiff like an onion. In reverse
order, we can build up its layers:

* A [`civil::Date`] is a plain Gregorian date, unattached to any specific
location.
* A [`civil::Time`] is a plain time on a clock, to nanosecond precision,
unattached to any specific location.
* A [`civil::DateTime`] combines a `civil::Date` and a `civil::Time` to provide
both a Gregorian date and a time on a clock. It is also unattached to any
specific location.
* A [`Timestamp`] is an unambiguous specific instant in time regardless of
geographic location, represented as the number of nanoseconds since the Unix
epoch.
* A [`TimeZone`] is a set of rules for mapping a `Timestamp` to a
`civil::DateTime` in a specific geographic region.
* A [`ZonedDateTime`] combines a `Timestamp`, a `civil::DateTime` and a
`TimeZone` to represent a specific instant in time in a specific geographic
location.

## Motivation

Whenever I needed datetime handling in Rust, I always got confused. Should I
use `chrono` or `time`? How do I get my local time from a timestamp? How do I
use my system's time zone database? How do I do calendar arithmetic?

Before I started working on Jiff, this confusion got annoying enough for me to
consider writing my own crate. I felt like the APIs of both `chrono` and `time`
could be improved upon. I personally found the domain quite interesting and so
set out to build something better.

Why not improve on an existing solution, like `chrono` or `time`, instead of
adding yet another choice into the ecosystem? I do think there are material
downsides of having too many choices, but I had concluded that both `chrono`
and `time` (in 2024) had reached a local maximum. Moreover, in 2024, I was not
an expert in datetime handling. It took building a datetime library to become
one. So convincing crate maintainers to go in a radically different direction
would have been a tall order.

Jiff's [design] and [comparison] documentation contain much more detail
regarding the motivation and specific examples between Jiff, `chrono` and
`time`.

## Inspiration

Jiff is heavily inspired by [Temporal], which is JavaScript's standard library
solution for date and time management. Jiff inherits API design, functionality
and even parsing/formatting of datetime values. Note though that Jiff does not
and does not aim to be a 100% compatible implementation of the Temporal
specification. Jiff is meant to be a Rust ecosystem library first and foremost.

With that said, if you're familiar with Temporal and Rust, then it's likely
you'll find Jiff to be a relatively straight-forward translation of Temporal's
API to Rust. This example shows how to use Temporal to round a calendar span
to a nearest number of days relative to a date:

```
const duration = Temporal.Duration.from({ months: 1, days: 1, hours: 1 });
const roundedDuration = duration.round({
  largestUnit: "days",
  smallestUnit: "days",
  relativeTo: Temporal.PlainDateTime.from("2022-01-01"),
});
console.log(roundedDuration); // "P32D"
```

And here's the equivalent operation in Jiff:

```rust
use jiff::{SpanRound, ToSpan, Unit};

fn main() -> anyhow::Result<()> {
    let span = 1.month().days(1).hours(1);
    let rounded_span = span.round(
        SpanRound::new()
            .largest(Unit::Day)
            .smallest(Unit::Day)
            .relative("2022-01-01".parse::<jiff::civil::Date>()?),
    )?;
    assert_eq!(rounded_span.to_string(), "P32D");
    Ok(())
}
```

The main difference between these examples is that Jiff's duration type is
named `Span` instead of `Duration`. This was done to avoid confusion with
Rust's standard library `std::time::Duration` type.

Here are some other differences:

* Temporal's `Instant` type in Jiff is named `Timestamp` in order to avoid
confusion with Rust's standard library `std::time::Instant` type.
* Temporal's "plain" date and time types are called "civil" in Jiff.
* Jiff only supports the Gregorian calendar.
* Jiff's datetime range only extends to +/- 9999, where as Temporal's range is
larger.

It's easy to say that without Temporal, Jiff wouldn't exist in its current
form today.

[Jiff]: https://docs.rs/jiff/1.*
[pit of success]: https://blog.codinghorror.com/falling-into-the-pit-of-success/
[adoption]: https://crates.io/crates/jiff/reverse_dependencies
[Temporal]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Temporal
[design]: https://docs.rs/jiff/1.*/jiff/_documentation/design/index.html
[comparison]: https://docs.rs/jiff/1.*/jiff/_documentation/comparison/index.html
[`civil::Date`]: https://docs.rs/jiff/1.*/jiff/civil/struct.Date.html
[`civil::DateTime`]: https://docs.rs/jiff/1.*/jiff/civil/struct.DateTime.html
[`civil::Time`]: https://docs.rs/jiff/1.*/jiff/civil/struct.Time.html
[`Timestamp`]: https://docs.rs/jiff/1.*/jiff/struct.Timestamp.html
[`TimeZone`]: https://docs.rs/jiff/1.*/jiff/tz/struct.TimeZone.html
[`ZonedDateTime`]: https://docs.rs/jiff/1.*/jiff/struct.ZonedDateTime.html
