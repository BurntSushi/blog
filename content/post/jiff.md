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
  * [Follow Along](#follow-along)
  * [The DateTime Onion](#the-datetime-onion)
  * [Defining a time zone](#defining-a-time-zone)
  * [Durations](#durations)
  * [When should I use Jiff?](#when-should-i-use-jiff)
* [Parsing and Formatting](#parsing-and-formatting)
  * [`FromStr` and `Display`](#fromstr-and-display)
  * [`strptime` and `strftime`](#strptime-and-strftime)
  * [The "friendly" Duration Format](#the-friendly-duration-format)
  * [The `jiff::fmt` module](#the-jifffmt-module)
  * [Locale Sensitive Formatting via ICU4X Integration](#locale-sensitive-formatting-via-icu4x-integration)
* [Date Manipulation](#date-manipulation)
  * [Date Basics](#date-basics)
  * [Date Arithmetic](#date-arithmetic)
  * [Date Rounding](#date-rounding)
  * [Other Calendars](#other-calendars)
* [Motivation](#motivation)
* [Inspiration](#inspiration)
* [The Jiff 1.0 Commitment](#the-jiff-10-commitment)

## Introduction

In programming, a datetime library's core functionality rests at the
intersection of a computer system's concept of time and a human's concept of
time. The prevailing notion of system time is [Unix time]. A human's concept
of time is far more complex, but the most popular such model is the [Gregorian
calendar] coupled with either a 12-hour or 24-hour clock. At the core of every
datetime library rests some routine for converting between these concepts.

A datetime library typically provides much more than this. Indeed, datetime
handling has [historically been very difficult][falsehoods]. My own belief is
that the difficulty comes from _poor modeling of the domain_. Instead of
encapsulating the complexity inside a reusable bundle of code, many datetime
libraries pass some or all of the complexity on to its users. Jiff instead
tries hard to encapsulate the complexity and make it difficult for users to
do something wrong. ([Jiff is not the first to do this!](#inspiration))

In this section of the blog, we'll do some very basic introductions.

### Follow Along

To follow along with the examples in this blog, create a new Cargo project
with dependencies on `jiff` and `anyhow`:

```
mkdir jiff-playground
cd jiff-playground
cargo init --bin
cargo add jiff anyhow
$EDITOR src/main.rs
```

Most examples are a complete self-contained Rust program. Just copy the code
and replace whatever is in `src/main.rs`. Then run the program:

```
cargo run
```

### Terms

I am loath to present a dictionary. However, the datetime domain is littered
with confusable terms, so I think it's important to orient ourselves on a
shared vocabulary. I've kept this section as small as possible. (Some terms,
like "time zone," are omitted here because they are best defined through
exploration in the sections that follow.)

* [Civil time]: The time you see on your clock. And in general, the time that
the humans in your approximate geographic vicinity also see. That is, civil
time is a human coordinated agreement for communicating time in a particular
geographic region. Civil time is also known as: local time, plain time, naive
time, clock time and others.
* [UTC]: An abbreviation for _Coordinated Universal Time_; it is a time
standard that serves as the basis for civil time around the world. It was
originally intended to roughly correspond to [mean solar time].
* [TAI]: An abbreviation for _International Atomic Time_; it is a time standard
as measured by atomic clocks. It isn't directly connected to mean solar time.
* [Leap second]: An adjustment to TAI to bring it closer to mean solar time
and thus forming the basis of UTC.
* [Unix time]: UTC, but ignoring leap seconds.

TAI and leap seconds won't be a feature of this blog, but they are listed here
in order to convey the difference between Unix time and UTC.

### The DateTime Onion

As a way to transition into some of the core types of Jiff, I've come to think
of its datetime types like an onion. In reverse order, we can build up
its layers all the way to a `ZonedDateTime`. So let's start with more terms,
but this time, they are also types in Jiff:

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
`civil::DateTime`, and vice versa, in a specific geographic region.
* A [`ZonedDateTime`] combines a `Timestamp`, a `civil::DateTime` and a
`TimeZone` to represent a specific instant in time in a specific geographic
location.

To make this a bit more concrete, we can see what this looks like in code.
Since a time zone can map from a civil datetime to an instant in time, one way
to create a zoned datetime requires only a civil datetime and a time zone:

```rust
use jiff::civil;

fn main() -> anyhow::Result<()> {
    let date = civil::date(2026, 11, 1);
    let time = civil::time(17, 30, 0, 0);
    let datetime = date.to_datetime(time);
    // zdt is an initialism for "ZonedDateTime"
    let zdt = datetime.in_tz("America/New_York")?;

    assert_eq!(zdt.timestamp().as_second(), 1793572200);

    Ok(())
}
```

Another way to create a zoned datetime requires only a timestamp and a time
zone. This is because time zones can _also_ map a timestamp to a particular
civil datetime:

```rust
use jiff::{civil, Timestamp};

fn main() -> anyhow::Result<()> {
    let timestamp = Timestamp::from_second(1793572200)?;
    let zdt = timestamp.in_tz("America/New_York")?;

    // This is a shortcut for creating a civil datetime.
    // The previous example spells it out a bit more verbosely
    // for clarity.
    let expected = civil::date(2026, 11, 1).at(17, 30, 0, 0);
    assert_eq!(zdt.datetime(), expected);

    Ok(())
}
```

Notice how we get the equivalent zoned datetime in both cases, but we come at
it from different directions. In the first case, we start with a time that we
might see on our clocks and calendars. In the second case, we start with a
particular instant in time that we might get from a database or file system or
some other system call.

### Defining a time zone

There is one piece missing from the zoned datetime onion examples: the
time zone _offset_. An offset represents the amount of time one must
adjust UTC time in order to get civil time. For example, consider the time
`2026-11-01T22:30:00` in UTC. In order to get the civil datetime for someone
living in New York, one would subtract 5 hours:

```ignore
2026-11-01T22:30:00 - 05:00 = 2026-11-01T17:30:00
```

Indeed, every zoned datetime has an offset associated with it:

```rust
use jiff::{civil, tz, Timestamp};

fn main() -> anyhow::Result<()> {
    let timestamp = Timestamp::from_second(1793572200)?;
    let zdt = timestamp.in_tz("America/New_York")?;
    assert_eq!(zdt.offset().seconds(), -(5 * 60 * 60));

    let dt = civil::date(2026, 11, 1).at(17, 30, 0, 0);
    let zdt = dt.in_tz("America/New_York")?;
    assert_eq!(zdt.offset(), tz::offset(-5));

    Ok(())
}
```

> **NOTE**: The `America/New_York` string you see in the above example is an
> IANA Time Zone Database Identifier. The [IANA Time Zone Database] is a
> collection of time zone definitions that can be referenced by a short
> identifier string. Examples: `Australia/Sydney`, `Asia/Tokyo`,
> `Europe/Berlin`.

When a civil datetime is combined with an offset, then what you get is always
an instant in time.

With the concept of an offset introduced, we can now give a more precise
definition of a time zone. A time zone **is a geographic region in which civil
clocks agree** and which provides the following:

* A mapping from an instant in time to an offset from UTC. The offset applied
to an instant yields a civil datetime.
* A mapping from a civil datetime to a possibly ambiguous offset from UTC.
The offset applied to a civil datetime yields an instant.

Both mappings have been demonstrated in prior examples, but the mapping
from civil datetime to an offset seems to be rather complicated due to some
potential ambiguity. When does that ambiguity arise? An ambiguous offset occurs
when the civil datetime either doesn't correspond to a valid clock time in this
time zone (e.g., when clocks are moved ahead for [daylight saving time]) or
where there could be multiple valid clock times (e.g., when clocks are moved
backward for [daylight saving time]).

Let's look at some examples. In this first example, we'll take a look at times
around the daylight saving time transition from summer time to standard time
in New York. In New York, the clocks were "set back" by an hour at exactly 2AM.
This caused the 1AM hour to repeat itself.

```rust
use jiff::{
    civil::date,
    tz::{offset, TimeZone},
};

fn main() -> anyhow::Result<()> {
    let tz = TimeZone::get("America/New_York")?;

    // During summer time, New York's offset from UTC is -04:00.
    let zdt = date(2026, 11, 1).at(0, 30, 0, 0).to_zoned_datetime(tz.clone())?;
    assert_eq!(zdt.offset(), offset(-4));

    // During standard time, New York's offset from UTC is -05:00.
    let zdt = date(2026, 11, 1).at(2, 30, 0, 0).to_zoned_datetime(tz.clone())?;
    assert_eq!(zdt.offset(), offset(-5));

    // But what does 01:30AM mean? Is it the first repetition or the second?
    // Jiff automatically chooses the first by default.
    let zdt = date(2026, 11, 1).at(1, 30, 0, 0).to_zoned_datetime(tz.clone())?;
    assert_eq!(zdt.offset(), offset(-4));

    Ok(())
}
```

The above example shows how Jiff resolves the ambiguity when a "fold" occurs,
or when there is a time zone transition that moves the civil time backwards.
But we can actually dig down into lower level APIs to change how that ambiguity
is resolved. By default, Jiff uses a "compatible" strategy that always selects
the first repetition of a time in a fold. But other strategies can be selected
by using APIs on a `TimeZone` directly:

```rust
use jiff::{
    civil::date,
    tz::{offset, TimeZone},
};

fn main() -> anyhow::Result<()> {
    let tz = TimeZone::get("America/New_York")?;
    let dt = date(2026, 11, 1).at(1, 30, 0, 0);

    // The same that Jiff uses by default.
    let zdt = tz.to_ambiguous_zoned_datetime(dt).compatible()?;
    assert_eq!(zdt.offset(), offset(-4));

    // Always selects the earlier datetime.
    // For a fold, this is the same as the
    // default "compatible" strategy.
    let zdt = tz.to_ambiguous_zoned_datetime(dt).earlier()?;
    assert_eq!(zdt.offset(), offset(-4));

    // Always selects the later datetime.
    // For a fold, this selects the second
    // occurrence of the 1AM hour!
    let zdt = tz.to_ambiguous_zoned_datetime(dt).later()?;
    assert_eq!(zdt.offset(), offset(-5));

    // Always returns an error when there
    // is any ambiguity.
    assert_eq!(
        tz.to_ambiguous_zoned_datetime(dt)
            .unambiguous()
            .unwrap_err()
            .to_string(),
        "error converting datetime to instant in time zone America/New_York: \
         datetime is ambiguous \
         since it falls into a fold between offsets -04 and -05",
    );

    Ok(())
}
```

The same idea applies to _gaps_, or time zone transitions where there's some
civil time that gets skipped on civil clocks. For example, on March 8, 2026
at 2AM, civil time skipped ahead to 3AM:

```rust
use jiff::{
    civil::date,
    tz::{offset, TimeZone},
};

fn main() -> anyhow::Result<()> {
    let tz = TimeZone::get("America/New_York")?;
    let dt = date(2026, 3, 8).at(2, 30, 0, 0);

    // The "compatible" strategy selects
    // the later time in the case of gaps.
    let zdt = tz.to_ambiguous_zoned_datetime(dt).compatible()?;
    assert_eq!(zdt.offset(), offset(-4));

    // Always selects the earlier datetime.
    // For a gap, this selects the hour
    // before the gap.
    let zdt = tz.to_ambiguous_zoned_datetime(dt).earlier()?;
    assert_eq!(zdt.offset(), offset(-5));

    // Always selects the later datetime.
    // For a gap, this is the same as the
    // default "compatible" strategy.
    let zdt = tz.to_ambiguous_zoned_datetime(dt).later()?;
    assert_eq!(zdt.offset(), offset(-4));

    // Always returns an error when there
    // is any ambiguity.
    assert_eq!(
        tz.to_ambiguous_zoned_datetime(dt)
            .unambiguous()
            .unwrap_err()
            .to_string(),
        "error converting datetime to instant in time zone America/New_York: \
         datetime is ambiguous \
         since it falls into a gap between offsets -05 and -04",
    );

    Ok(())
}
```

### Durations

There are two remaining core types, both representing some notion of a
duration:

* [`Span`] is a powerful hybrid duration that combines both calendar units
(years, months, weeks, days) and physical time units (hours, minutes, seconds,
milliseconds, microseconds, nanoseconds). It is a beefy type that preserves the
individual unit values.
* [`SignedDuration`] is a signed version of [`std::time::Duration`] and always
corresponds to a 96-bit integer number of nanoseconds. It can only represent
physical time (units of hours or smaller) and not calendar units (units of days
or greater).

A `Span` is the preferred duration type to use whenever possible. Here's a
simple example that shows how to get the number of days between two civil
dates:

```rust
use jiff::civil::date;

fn main() -> anyhow::Result<()> {
    let d1 = date(2024, 2, 1);
    let d2 = date(2024, 3, 1);
    let span = d2 - d1;
    assert_eq!(span.get_days(), 29);
    // or, equivalently:
    assert_eq!(d1.until(d2)?.get_days(), 29);

    // Flipping d1 and d2 gives the same result,
    // but negative.
    let span = d1 - d2;
    assert_eq!(span.get_days(), -29);

    Ok(())
}
```

Durations can also be added to dates:

```rust
use jiff::{civil::date, ToSpan};

fn main() -> anyhow::Result<()> {
    let d1 = date(2024, 2, 1);
    // This convenient syntax comes from the
    // `ToSpan` extension trait.
    let span = 29.days();
    assert_eq!(d1.checked_add(span)?, date(2024, 3, 1));

    Ok(())
}
```

Generally speaking, whenever there is an operation involving a `Span`, there is
an equivalent operation available with `SignedDuration`:

```rust
use jiff::{civil::date, SignedDuration};

fn main() -> anyhow::Result<()> {
    let dt1 = date(2024, 2, 1).at(17, 30, 0, 0);
    let sdur = SignedDuration::from_mins(90);
    assert_eq!(dt1.checked_add(sdur)?, date(2024, 2, 1).at(19, 0, 0, 0));

    Ok(())
}
```

Notice that in the above example, we actually switched from using a
`civil::Date` to a `civil::DateTime`. This was done to show how physical time
units can be added. But note that the same can be accomplished with a `Span`,
because it combines all units:

```rust
use jiff::{civil::date, ToSpan};

fn main() -> anyhow::Result<()> {
    let dt1 = date(2024, 2, 1).at(17, 30, 0, 0);
    let span = 1.hour().minutes(30);
    assert_eq!(dt1.checked_add(span)?, date(2024, 2, 1).at(19, 0, 0, 0));

    // Calendar and physical time units can
    // be combined!
    let span = 1.month().hours(1).minutes(30);
    assert_eq!(dt1.checked_add(span)?, date(2024, 3, 1).at(19, 0, 0, 0));

    Ok(())
}
```

In brief, use a `Span` when you can. But when it's too bulky or too slow, use
a `SignedDuration`. One of the key benefits of a `Span` is that since it
represents each unit value distinctly, one can preserve the original meaning
of a duration. For example, it's common to represent the runtime of films in
minutes, even when the duration exceeds one hour. Indeed, a span of 2 hours
is a distinct span from a span of 120 minutes:

```rust
use jiff::ToSpan;

fn main() -> anyhow::Result<()> {
    let span1 = 2.hours();
    let span2 = 120.minutes();
    // This means the spans are distinct values
    // in memory.
    assert_ne!(span1.fieldwise(), span2.fieldwise());

    Ok(())
}
```

The [`Span::fieldwise`] routine converts a `Span` into a value whose units can
be compared in a field-by-field fashion. To compare spans based on the actual
physical duration they represent, use [`Span::compare`]:

```rust
let span1 = 2.hours();
let span2 = 120.minutes();
assert!(span1.compare(span2)?.is_eq());
```

This was a very brief introduction to durations in Jiff. They are _a lot_ more
powerful than what is shown here. You might even try playing with the examples
above to explore its nooks and crannies. We'll cover their API in more detail
in a later section.

### When should I use Jiff?

Jiff isn't always the right thing to use. Sometimes you can get by with
something simpler.

First and foremost, if you're _measuring_ the time something takes in a
program, you should almost certainly be using monotonic time. This is
available via [`std::time::Instant`]:

```rust
use std::time::{Duration, Instant};

fn main() -> anyhow::Result<()> {
    let start = Instant::now();

    // do some serious work
    std::thread::sleep(Duration::from_secs(1));

    let elapsed = Instant::now().duration_since(start);
    println!("{elapsed:?}");

    Ok(())
}
```

If you do need to compute elapsed time using your system clock, then Jiff
is very convenient for that:

```rust
use std::time::Duration;

use jiff::Timestamp;

fn main() -> anyhow::Result<()> {
    // assume `start` came from somewhere
    // over a network
    let start = Timestamp::now();
    std::thread::sleep(Duration::from_secs(1));

    let elapsed = Timestamp::now().duration_since(start);
    println!("{elapsed}");

    Ok(())
}
```

But you don't actually need Jiff for this. A [`std::time::SystemTime`] will
work perfectly fine:

```rust
use std::time::{Duration, SystemTime};

fn main() -> anyhow::Result<()> {
    // assume `start` came from somewhere
    // over a network
    let start = SystemTime::now();
    std::thread::sleep(Duration::from_secs(1));

    let elapsed = SystemTime::now().duration_since(start)?;
    println!("{elapsed:?}");

    Ok(())
}
```

Where Jiff starts to become useful is when you need something beyond just an
integer number of seconds since the Unix epoch. For example, some very hairy
math is required in order to convert a `SystemTime` into an RFC 3339 timestamp
like `2026-11-14T21:34:00Z`.

As long as you can live in UTC completely, don't care about local time and can
work with Unix timestamps only, then the Rust standard library won't steer you
wrong.

## Parsing and Formatting

Conversion to and from strings is a major part of Jiff. Jiff's focus is
primarily on interoperability via the following:

* [RFC 3339] specifies a format for fixed offset instants in time, in terms of
a human readable civil datetime. For example, `2026-11-12T15:36:00-05:00`. This
is the serialization format used by [`Timestamp`].
* [RFC 9557] supersedes RFC 3339 to add time zone information. For example,
`2026-11-12T15:36:00-05:00[America/New_York]`. This is the serialization format
used by [`ZonedDateTime`].
* [ISO 8601] covers mostly everything else, including durations and
civil datetimes. For example, `P5Y3MT20H` is an ISO 8601 duration and
`2026-11-12T15:30:00` is an ISO 8601 civil datetime whose corresponding instant
in time is unknown.

What Jiff actually implements is the
[Temporal RFC 9557 / ISO 8601] grammar for flexible parsing of datetimes from
strings. For most use cases, it suffices to think in terms of RFC 3339,
RFC 9557 or ISO 8601. But Jiff does support parsing strings that aren't valid
in any of those three specifications. For example, `2026-11-12[America/New_York]`
parses as if it was `2026-11-12T00:00:00[America/New_York]`.

In this section, we'll take a tour of how Jiff does datetime formatting and
printing.

### `FromStr` and `Display`

Every datetime and duration type in Jiff implements the standard library
[`FromStr`] and [`Display`] traits. This means that it's natural to parse
datetimes:

```rust
let ts: Timestamp = "2026-10-31T05:15:00Z".parse()?;
```

and print them:

```rust
println!("{ts}");
```

Parsing is flexible, so even if there are components missing, parsing still
succeeds. For example, without minutes or seconds, those components default to
zero:

```rust
let dt: civil::DateTime = "2026-10-31T05".parse()?;
assert_eq!(dt.to_string(), "2026-10-31T05:00:00");
```

Another example of flexible parsing is that if a string _contains_ another date
type, then one can parse that. For example, one can parse a date out of an
ISO 8601 datetime:

```rust
let d: civil::Date = "2026-10-31T05:30".parse()?;
assert_eq!(d.to_string(), "2026-10-31");
```

A [`ZonedDateTime`] always uses [RFC 9557] syntax, which includes the IANA time
zone identifier:

```rust
let zdt: ZonedDateTime = "2026-10-31T05:30[Australia/Sydney]".parse()?;
// While the string we parsed didn't have any offset,
// one was automatically added by consulting the time
// zone definition for the civil datetime provided.
assert_eq!(zdt.to_string(), "2026-10-31T05:30:00+11:00[Australia/Sydney]");
```

If you want to print a zoned datetime using only [RFC 3339] syntax, then you
can do so by formatting the timestamp directly with the zoned datetime's
offset:

```rust
let zdt: ZonedDateTime = "2026-10-31T05:30[Australia/Sydney]".parse()?;
assert_eq!(
    zdt.timestamp().display_with_offset(zdt.offset()).to_string(),
    "2026-10-31T05:30:00+11:00",
);
```

The [`Span`] and [`SignedDuration`] types also support the `FromStr` and
`Display` traits via the ISO 8601 duration format. For example:

```rust
use jiff::{SignedDuration, Span};

fn main() -> anyhow::Result<()> {
    let duration = "PT5H30M";

    let span: Span = duration.parse()?;
    assert_eq!(span.to_string(), "PT5H30M");

    let sdur: SignedDuration = duration.parse()?;
    assert_eq!(sdur.to_string(), "PT5H30M");

    Ok(())
}
```

Since a `Span` is a hybrid duration that also supports calendar units, it can
parse durations with calendar units and physical time units. A `SignedDuration`
can only parse durations with physical time units:

```rust
use jiff::{SignedDuration, Span};

fn main() -> anyhow::Result<()> {
    let duration = "P11Y2M28DT5H30M";

    let span: Span = duration.parse()?;
    assert_eq!(span.to_string(), "P11Y2M28DT5H30M");

    let err = duration.parse::<SignedDuration>().unwrap_err();
    assert_eq!(
        err.to_string(),
        "parsing ISO 8601 duration in this context requires \
         that the duration contain a time component \
         and no components of days or greater",
    );

    Ok(())
}
```

The ISO 8601 duration format does, however, have some shortcomings that we'll
discuss in the next section.

### The "friendly" Duration Format

Because ISO 8601 duration strings enjoy broad support, it is the default in
Jiff for both `Span` and `SignedDuration`. However, it has two important
shortcomings:

1. It is difficult to read. For example, in `P11Y2M28DT5H30M`, the lack of
whitespace makes the unit values and the unit labels all run together.
2. An ISO 8601 duration cannot unambiguously represent all possible `Span`
values.

The second shortcoming is best seen with an example. Consider a `Span` that is
2000 milliseconds. If you convert this to an ISO 8601 duration string and then
parse it back, you end up with 2 seconds instead:

```rust
use jiff::{Span, ToSpan};

fn main() -> anyhow::Result<()> {
    let original = 2000.milliseconds();
    assert_eq!(original.to_string(), "PT2S");

    let got: Span = "PT2S".parse()?;

    // The spans have different unit values!
    assert_eq!(original.get_seconds(), 0);
    assert_eq!(original.get_milliseconds(), 2000);
    assert_eq!(got.get_seconds(), 2);
    assert_eq!(got.get_milliseconds(), 0);

    Ok(())
}
```

This might not be desirable for cases where you want a span's string format
to faithfully represent all possible distinct values of a span.

For these two reasons, Jiff comes with its own bespoke "friendly" duration
format. The friendly format is non-lossy _and_ is meant to be easier to read
for English speakers than the ISO 8601 duration format. Here are some examples
that are all valid friendly durations:

```
40d
40 days
1y1d
1yr 1d
3d4h59m
3 days, 4 hours, 59 minutes
3d 4h 59m
2h30m
2h 30m
1mo
1w
1 week
1w4d
1 wk 4 days
1m
0.0021s
0s
0d
0 days
3 mins 34s 123ms
3 mins 34.123 secs
3 mins 34,123s
1y1mo1d1h1m1.1s
1yr 1mo 1day 1hr 1min 1.1sec
1 year, 1 month, 1 day, 1 hour, 1 minute 1.1 seconds
1 year, 1 month, 1 day, 01:01:01.1
```

Parsing a friendly duration is as easy as parsing an ISO 8601 duration: the
`FromStr` implementations on both `Span` and `SignedDuration` support ISO 8601
durations and friendly durations simultaneously:

```rust
let span: Span = "6y 10mo 3d 23h".parse()?;
// Recall that `Span::fieldwise` permits
// equality comparisons on `Span` in a
// unit-by-unit fashion.
assert_eq!(span.fieldwise(), 6.years().months(10).days(3).hours(23));
```

Printing a friendly duration does require a bit more effort, since it isn't
the default. Instead of `Span::to_string()`, one can use the "alternative"
syntax via Rust's formatting machinery:

```rust
let span = 2000.milliseconds();
// This also demonstrates that our unit
// values are preserved.
assert_eq!(format!("{span:#}"), "2000ms");
```

One can also use a lower level friendly printer to customize its output.
For example, here's how to write a span in the friendly format to an existing
`String` using more spacing, longer unit labels and commas separating each
unit:

```rust
use jiff::{fmt::friendly, ToSpan};

fn main() -> anyhow::Result<()> {
    static PRINTER: friendly::SpanPrinter = friendly::SpanPrinter::new()
        .comma_after_designator(true)
        .spacing(friendly::Spacing::BetweenUnitsAndDesignators)
        .designator(friendly::Designator::Verbose);

    let span = 6.years().months(10).days(3).hours(23);
    let mut buf = String::new();
    PRINTER.print_span(&span, &mut buf)?;
    assert_eq!(buf, "6 years, 10 months, 3 days, 23 hours");

    Ok(())
}
```

See the [`jiff::fmt::friendly`] module for a lot more detail, including a
grammar that precisely specifies the format supported.

### `strptime` and `strftime`

The [POSIX `strptime`] and [POSIX `strftime`] routines are far from the ideal
way to format and parse datetimes, but they are ubiquitous and most programmers
are familiar with them. Because of their ubiquity, and because of the utility
of being able to format and parse bespoke datetime formats, Jiff provides
implementations of these routines that are mostly POSIX compatible.

Every datetime type in Jiff comes with `strptime` and `strftime` routines.
For example, to parse a civil date in the US `month/day/year` format:

```rust
let date = civil::Date::strptime("%m/%d/%Y", "11/13/2026")?;
assert_eq!(date.to_string(), "2026-11-13");
```

Or to format a zoned datetime in a convenient to read representation for
English speakers:

```rust
let zdt = civil::date(2026, 11, 13).at(17, 30, 0, 0).in_tz("Australia/Sydney")?;
assert_eq!(
    zdt.strftime("%a, %b %d, %Y %H:%M %Z").to_string(),
    "Fri, Nov 13, 2026 17:30 AEDT",
);
```

Callers can also use lower level APIs from [`jiff::fmt::strtime`] to get access
to each parsed component individually:

```rust
use jiff::{civil, fmt::strtime::BrokenDownTime};

fn main() -> anyhow::Result<()> {
    let tm = BrokenDownTime::parse(
        "%a, %b %d, %Y %H:%M",
        "Fri, Nov 13, 2026 17:30",
    )?;
    assert_eq!(tm.weekday(), Some(civil::Weekday::Friday));
    assert_eq!(tm.month(), Some(11));
    assert_eq!(tm.day(), Some(13));
    assert_eq!(tm.year(), Some(2026));
    assert_eq!(tm.hour(), Some(17));
    assert_eq!(tm.minute(), Some(30));
    assert_eq!(tm.second(), None);

    Ok(())
}
```

### The `jiff::fmt` module

While the above tour should cover most use cases, there are sometimes reasons
for exerting more control over parsing or formatting. Or for supporting
alternate formats like RFC 2822. The [`jiff::fmt`] module is meant to cover
these use cases at the cost of more complex APIs. For example, consider parsing
an [RFC 2822] datetime string from a `&[u8]` (which lets callers avoid a UTF-8
validation check):

```rust
use jiff::fmt::rfc2822;

fn main() -> anyhow::Result<()> {
    static PARSER: rfc2822::DateTimeParser = rfc2822::DateTimeParser::new();

    let data = b"Sat, 13 Jul 2024 15:09:59 -0400";
    let ts = PARSER.parse_timestamp(data)?;
    assert_eq!(ts.to_string(), "2024-07-13T19:09:59Z");
    Ok(())
}
```

Another example shows how to change the default disambiguation strategy when
the provided civil datetime is ambiguous. For example, Sydney, Australia moved
the clocks backwards at `2026-04-05T03:00:00` local time. That is, they went
from UTC offset `+11` to UTC offset `+10`. So what happens if we try to parse
a time on the clock that is repeated as a result?

```rust
use jiff::ZonedDateTime;

fn main() -> anyhow::Result<()> {
    let zdt: ZonedDateTime = "2026-04-05T02:30[Australia/Sydney]".parse()?;
    assert_eq!(zdt.to_string(), "2026-04-05T02:30:00+11:00[Australia/Sydney]");
    Ok(())
}
```

Here, the first occurrence of `02:30` is selected, just before Sydney leaves
daylight saving time. Now, the data itself can actually select the desired
offset:

```rust
use jiff::ZonedDateTime;

fn main() -> anyhow::Result<()> {
    let zdt: ZonedDateTime =
        "2026-04-05T02:30+11:00[Australia/Sydney]".parse()?;
    assert_eq!(zdt.to_string(), "2026-04-05T02:30:00+11:00[Australia/Sydney]");

    let zdt: ZonedDateTime =
        "2026-04-05T02:30+10:00[Australia/Sydney]".parse()?;
    assert_eq!(zdt.to_string(), "2026-04-05T02:30:00+10:00[Australia/Sydney]");

    Ok(())
}
```

But this happens because _there is no ambiguity_ in the data. Since the offset
is supplied, the exact instant in time is known. Going back to
`2026-04-05T02:30[Australia/Sydney]`, which _is_ ambiguous because it could
refer to two different instants in time, how do we modify the strategy for
resolving that ambiguity? We can't using the standard library `FromStr` trait,
so we instead have to construct our own parser manually, just like we did for
[RFC 2822] parsing above. This example shows how to do that and all of the
different disambiguation options. Remember, UTC offset +11:00 corresponds to
the first occurrence of `2026-04-05T02:30` on the clocks of folks in Sydney
while UTC offset +10:00 corresponds to the second occurrence.

```rust
use jiff::{fmt::temporal::DateTimeParser, tz::Disambiguation};

fn main() -> anyhow::Result<()> {
    let string = "2026-04-05T02:30[Australia/Sydney]";

    // "compatible" is the default strategy, which selects
    // the earlier time in a fold (usually moving from daylight
    // saving time to standard time) and the later time in a
    // gap (usually moving from standard time to daylight
    // saving time).
    let parser = DateTimeParser::new().disambiguation(Disambiguation::Compatible);
    let zdt = parser.parse_zoned_datetime(string)?;
    assert_eq!(zdt.to_string(), "2026-04-05T02:30:00+11:00[Australia/Sydney]");

    // "earlier" always selects the first occurrence
    // in a fold, or the time before a gap.
    // So this is the same as compatible in this case,
    // since our civil time is in a fold.
    let parser = DateTimeParser::new().disambiguation(Disambiguation::Earlier);
    let zdt = parser.parse_zoned_datetime(string)?;
    assert_eq!(zdt.to_string(), "2026-04-05T02:30:00+11:00[Australia/Sydney]");

    // "later" always selects the second occurrence
    // in a fold, or the time after a gap.
    // So this is different from "compatible" for a
    // fold. Notice that the UTC +10:00 offset is
    // chosen.
    let parser = DateTimeParser::new().disambiguation(Disambiguation::Later);
    let zdt = parser.parse_zoned_datetime(string)?;
    assert_eq!(zdt.to_string(), "2026-04-05T02:30:00+10:00[Australia/Sydney]");

    // "reject" means that if the instant is ambiguous,
    // then an error is returned.
    let parser = DateTimeParser::new().disambiguation(Disambiguation::Reject);
    let err = parser.parse_zoned_datetime(string).unwrap_err();
    assert_eq!(
        err.to_string(),
        "error converting datetime to instant \
         in time zone Australia/Sydney: \
         datetime is ambiguous since it falls \
         into a fold between offsets +11 and +10",
    );

    Ok(())
}
```

The [`jiff::fmt`] module contains lots of other goodies that we won't cover
here, including, but not limited to:

* The ability to format directly into an existing allocation.
* Parsing a datetime into its individual components.
* Helpers for [`serde`] integration.

### Locale Sensitive Formatting via ICU4X Integration

No section on datetime formatting would be complete without addressing locales.
In this context, a locale is a set of user preferences used for formatting
datetimes. This is useful when you want to present a datetime to an end user
that doesn't expect standard formats from [RFC 3339], [RFC 9557] or [ISO 8601].

Jiff itself does not provide any facilities for dealing with locales. Instead,
users can opt into this support via the [ICU4X] project. To make conversions
between Jiff and ICU4X, the [`jiff-icu`] integration crate can be used.

Here's an example showing how to format a zoned datetime in the `fr-LA` locale:
(You'll need to run `cargo add icu jiff-icu` to your project for this to work.)

```rust
use icu::{
    calendar::Iso,
    datetime::{fieldsets, DateTimeFormatter},
    locale::locale,
    time::{zone::models::AtTime, TimeZoneInfo},
};
use jiff::ZonedDateTime;
use jiff_icu::ConvertFrom as _;

fn main() -> anyhow::Result<()> {
    // Firstly, do a conversion from Jiff's zoned datetime to ICU's.
    let jiff_zdt: ZonedDateTime =
        "2024-09-10T23:37:20-04[America/New_York]".parse()?;
    let icu_zdt =
        icu::time::ZonedDateTime::<Iso, TimeZoneInfo<AtTime>>::convert_from(
            &jiff_zdt,
        );

    // Second, create a datetime formatter for the fr-LA locale:
    let formatter = DateTimeFormatter::try_new(
        locale!("fr-LA").into(),
        fieldsets::YMDET::medium().with_zone(fieldsets::zone::SpecificLong),
    )?;

    // Thirdly, format the zoned datetime.
    assert_eq!(
        formatter.format(&icu_zdt).to_string(),
        "mar. 10 sept. 2024, 23:37:20 heure d’été de l’Est nord-américain",
    );

    Ok(())
}
```

The ICU4X API has a lot of options for customizing datetime formatting.
Exploring it further is outside the scope of this blog.

## Date Manipulation

Jiff provides many methods on its datetime types to make using them as
comfortable and error-free as possible.

### Date Basics

Every datetime type provides a `new` constructor that lets callers build it
from its constituent parts. In most cases, this constructor is fallible. For
example:

```rust
use jiff::civil::Date;

fn main() -> anyhow::Result<()> {
    let d = Date::new(2024, 2, 29)?;
    // All datetime types have convenient
    // accessors for each of their components.
    assert_eq!(d.year(), 2024);
    assert_eq!(d.month(), 2);
    assert_eq!(d.day(), 29);

    let err = Date::new(2025, 2, 29).unwrap_err();
    assert_eq!(
        err.to_string(),
        "parameter 'day' for `2025-02` is invalid, must be in range `1..=28`",
    );

    Ok(())
}
```

Because creating dates is so common, there is a shorthand for doing so when
you know the inputs are valid:

```rust
let d = const { civil::date(2024, 2, 29) };
assert_eq!(d.year(), 2024);
assert_eq!(d.month(), 2);
assert_eq!(d.day(), 29);
```

That is, `civil::date` is a const function that will panic if its inputs are
invalid. It should only be used with inputs that are known to be valid.
It would, for example, be inappropriate to use this with user provided values.
Because of the `const` block above, if you try to provide invalid values,
you'll get a compiler error.

In addition to constructors, new date values can be constructed from old
date values using the `Date::with` API:

```rust
let d1 = const { civil::date(2024, 2, 29) };
// This API doesn't break just because 31
// is supplied before a month that has 31
// days. Any order is fine! This is because
// errors are deferred until `build()` is
// called.
let d2 = d1.with().day(31).month(3).build()?;
assert_eq!(d2.to_string(), "2024-03-31");
```

Once you have a date, there are oodles of APIs available:

```rust
let d = civil::date(2025, 7, 10);
assert_eq!(d.weekday(), civil::Weekday::Thursday);
assert_eq!(d.first_of_year(), civil::date(2025, 1, 1));
assert_eq!(d.last_of_year(), civil::date(2025, 12, 31));
assert_eq!(d.first_of_month(), civil::date(2025, 7, 1));
assert_eq!(d.last_of_month(), civil::date(2025, 7, 31));
assert_eq!(d.yesterday()?, civil::date(2025, 7, 9));
assert_eq!(d.tomorrow()?, civil::date(2025, 7, 11));
```

Moreover, because of the [datetime onion](#the-datetime-onion), all of these
APIs are also available on [`civil::DateTime`] and [`ZonedDateTime`]:

```rust
let create_zdt =
    |date: civil::Date| -> Result<ZonedDateTime, jiff::Error> {
        date.at(0, 0, 0, 0).in_tz("America/New_York")
    };

let zdt = create_zdt(civil::date(2025, 7, 10))?;
assert_eq!(zdt.weekday(), civil::Weekday::Thursday);
assert_eq!(zdt.first_of_year()?, create_zdt(civil::date(2025, 1, 1))?);
assert_eq!(zdt.last_of_year()?, create_zdt(civil::date(2025, 12, 31))?);
assert_eq!(zdt.first_of_month()?, create_zdt(civil::date(2025, 7, 1))?);
assert_eq!(zdt.last_of_month()?, create_zdt(civil::date(2025, 7, 31))?);
assert_eq!(zdt.yesterday()?, create_zdt(civil::date(2025, 7, 9))?);
assert_eq!(zdt.tomorrow()?, create_zdt(civil::date(2025, 7, 11))?);
```

These routines on `ZonedDateTime` also take time zone transitions into account.
For example, `ZonedDateTime` knows that `tomorrow()` doesn't always add 24
hours:

```rust
// On 2024-11-03 at 2AM in New York, time was rolled
// back 1 hour in the transition from daylight saving
// time to standard time. That means 2024-11-03 was
// actually 25 hours long in New York.
let zdt =
    civil::date(2024, 11, 2).at(17, 25, 0, 0).in_tz("America/New_York")?;
let tomorrow = zdt.tomorrow()?;
assert_eq!(tomorrow.datetime(), civil::date(2024, 11, 3).at(17, 25, 0, 0));

// The physical time difference between them is 25 hours!
let span = tomorrow - zdt;
assert_eq!(format!("{span:#}"), "25h");
```

> **NOTE**: We'll talk about date arithmetic in the next section.

When `ZonedDateTime::tomorrow()` (among other APIs that produce a
`ZonedDateTime`) would land in a gap or a fold, the "compatible" disambiguation
strategy is chosen automatically:

```rust
// For a zoned datetime that lands
// in a fold, the earlier instant is
// chosen by default.
let zdt =
    civil::date(2024, 11, 2).at(1, 30, 0, 0).in_tz("America/New_York")?;
let tomorrow = zdt.tomorrow()?;
assert_eq!(
    tomorrow.to_string(),
    "2024-11-03T01:30:00-04:00[America/New_York]"
);

// And for a gap, the later instant
// is chosen by default.
let zdt =
    civil::date(2024, 3, 9).at(2, 30, 0, 0).in_tz("America/New_York")?;
let tomorrow = zdt.tomorrow()?;
assert_eq!(
    tomorrow.to_string(),
    "2024-03-10T03:30:00-04:00[America/New_York]"
);
```

To override this behavior, you can drop down to civil datetimes, perform the
operation, and then use `TimeZone::to_ambiguous_zoned_datetime` to control the
exact disambiguation strategy:

```rust
// For a zoned datetime that lands
// in a fold, the earlier instant is
// chosen by default. But let's say
// we want the later instant.
let zdt =
    civil::date(2024, 11, 2).at(1, 30, 0, 0).in_tz("America/New_York")?;
let tomorrow = zdt
    .time_zone()
    .to_ambiguous_zoned_datetime(zdt.datetime().tomorrow()?)
    .later()?;
assert_eq!(
    tomorrow.to_string(),
    // Notice that the offset is now -05,
    // which corresponds to the later
    // instant.
    "2024-11-03T01:30:00-05:00[America/New_York]"
);
```

There are a few other methods of note on date types that are worth mentioning.
First is the ability to query for things like, "The Thursday after next" or
"The last Sunday of the month." The former can be answered by the `nth_weekday`
API:

```rust
let zdt1 = civil::date(2024, 7, 10).in_tz("America/New_York")?;
assert_eq!(zdt1.weekday(), civil::Weekday::Wednesday);
// Not this Thursday, but the next!
let zdt2 = zdt1.nth_weekday(2, civil::Weekday::Thursday)?;
assert_eq!(zdt2.weekday(), civil::Weekday::Thursday);
assert_eq!(zdt2.date(), civil::date(2024, 7, 18));
```

While the latter can be answered by the `nth_weekday_of_month` API:

```rust
let zdt1 = civil::date(2024, 7, 10).in_tz("America/New_York")?;
assert_eq!(zdt1.weekday(), civil::Weekday::Wednesday);
// The last Sunday of this month.
let zdt2 = zdt1.nth_weekday_of_month(-1, civil::Weekday::Sunday)?;
assert_eq!(zdt2.weekday(), civil::Weekday::Sunday);
assert_eq!(zdt2.date(), civil::date(2024, 7, 28));
```

Finally, users can access a convenient time series iterator via `series`:

```rust
use jiff::{civil, ToSpan};

fn main() -> anyhow::Result<()> {
    let start = civil::date(2024, 7, 10).in_tz("America/New_York")?;
    let mut dates = vec![];
    // Every week for 8 weeks.
    for zdt in start.series(1.week()).take(8) {
        dates.push(zdt);
    }
    assert_eq!(
        dates,
        vec![
            civil::date(2024, 7, 10).in_tz("America/New_York")?,
            civil::date(2024, 7, 17).in_tz("America/New_York")?,
            civil::date(2024, 7, 24).in_tz("America/New_York")?,
            civil::date(2024, 7, 31).in_tz("America/New_York")?,
            civil::date(2024, 8, 7).in_tz("America/New_York")?,
            civil::date(2024, 8, 14).in_tz("America/New_York")?,
            civil::date(2024, 8, 21).in_tz("America/New_York")?,
            civil::date(2024, 8, 28).in_tz("America/New_York")?,
        ]
    );

    Ok(())
}
```

All of these methods are also available on [`civil::Date`] and
[`civil::DateTime`], which should make it very natural to move between them.

### Date Arithmetic

All datetime types in Jiff support adding and subtracting durations, and
finding the span of time between two datetime values. Let's start with a
[`civil::Date`]:

```rust
use jiff::{civil, ToSpan};

fn main() -> anyhow::Result<()> {
    let d1 = civil::date(2024, 7, 30);
    let d2 = d1 + 2.days();
    assert_eq!(d2, civil::date(2024, 8, 1));

    Ok(())
}
```

Here we make use of the `std::ops::Add` trait implementation for `civil::Date`.
The downside of using this, though, is that if the date value goes out of
range, then it panics. That may or may not be acceptable. But if you're dealing
with user inputs, then you should use `Date::checked_add` instead:

```rust
use jiff::{civil, ToSpan};

fn main() -> anyhow::Result<()> {
    let d1 = civil::date(2024, 7, 30);
    let d2 = d1.checked_add(2.days())?;
    assert_eq!(d2, civil::date(2024, 8, 1));

    Ok(())
}
```

Let's take a look at a few other examples involving months and years:

```rust
let d = civil::date(2024, 5, 31);
assert_eq!(d.checked_add(1.month())?, civil::date(2024, 6, 30));
assert_eq!(d.checked_add(2.months())?, civil::date(2024, 7, 31));

let d = civil::date(2025, 1, 31);
assert_eq!(d.checked_add(1.month())?, civil::date(2025, 2, 28));

let d = civil::date(2024, 2, 29);
let span = 1.year().months(2).days(5);
assert_eq!(d.checked_add(span)?, civil::date(2025, 5, 4));
```

As with other methods on `Date`, they are also available on `ZonedDateTime`:

```rust
let d = civil::date(2024, 5, 31).at(13, 45, 0, 0).in_tz("America/New_York")?;
assert_eq!(
    d.checked_add(1.month())?,
    civil::date(2024, 6, 30).at(13, 45, 0, 0).in_tz("America/New_York")?
);
```

If we take a look at the type signature for [`ZonedDateTime::checked_add`], we
can see that it's actually polymorphic:

```rust
pub fn checked_add<A: Into<ZonedDateTimeArithmetic>>(
    &self,
    duration: A,
) -> Result<ZonedDateTime, Error>
```

Specifically, it accepts anything that can be converted into a
[`ZonedDateTimeArithmetic`]. In this case, there are `From` trait
implementations for the following types:

* [`Span`]
* [`SignedDuration`]
* [`std::time::Duration`]

This means that `ZonedDateTime::checked_add` can accept any one of these
duration values:

```rust
use jiff::{civil, SignedDuration};

fn main() -> anyhow::Result<()> {
    let d =
        civil::date(2024, 5, 31).at(13, 45, 0, 0).in_tz("America/New_York")?;
    assert_eq!(
        d.checked_add(SignedDuration::from_mins(45))?,
        civil::date(2024, 5, 31).at(14, 30, 0, 0).in_tz("America/New_York")?
    );

    Ok(())
}
```

This type of polymorphism is common in Jiff. Here, it is merely used to
provide a convenient way to use multiple duration types. But as we'll see,
it's also used to provide an extension point for configuration options that
control how the operation works.

In addition to adding or subtracting durations from datetime values, one can
also compute the duration between two datetime values. For example, to get the
number of days between two dates:

```rust
use jiff::{civil, ToSpan};

fn main() -> anyhow::Result<()> {
    let d1 = civil::date(2024, 5, 31);
    let d2 = civil::date(2024, 9, 30);
    assert_eq!(d2 - d1, 122.days().fieldwise());

    Ok(())
}
```

Similarly, we can use [`Date::until`] to compute the span between dates:

```rust
let d1 = civil::date(2024, 5, 31);
let d2 = civil::date(2024, 9, 30);
assert_eq!(d1.until(d2)?, 122.days().fieldwise());
```

Like with `Date::checked_add`, this routine is also available at
[`ZonedDateTime::until`]:

```rust
let zdt1 = civil::date(2024, 5, 31).in_tz("America/New_York")?;
let zdt2 = civil::date(2024, 9, 30).in_tz("America/New_York")?;
assert_eq!(zdt1.until(&zdt2)?, 2928.hours().fieldwise());
```

Notice though that hours, instead of days, are returned. This is because two
`ZonedDateTime` values might be in different time zones. Since a `ZonedDateTime`
is fundamentally an instant in time, and because days might be of varying length
depending on the time zone, defaulting to hours as the largest possible unit
means subtracting two zoned datetimes will always succeed. However, we can
request larger units in the duration returned by passing additional options.
Notably, `ZonedDateTime::until` is polymorphic:

```rust
pub fn until<'a, A: Into<ZonedDateTimeDifference<'a>>>(
    &self,
    other: A,
) -> Result<Span, Error>
```

That is, callers aren't just limited to passing a `ZonedDateTime` here, but
_anything_ that can be converted to a [`ZonedDateTimeDifference`]. Here is
one such example, where we can override the largest units returned in the
duration:

```rust
use jiff::{civil, ToSpan, Unit};

fn main() -> anyhow::Result<()> {
    let zdt1 = civil::date(2024, 5, 31).in_tz("America/New_York")?;
    let zdt2 = civil::date(2024, 9, 30).in_tz("America/New_York")?;
    assert_eq!(
        zdt1.until((Unit::Month, &zdt2))?,
        3.months().days(30).fieldwise()
    );

    Ok(())
}
```

A `ZonedDateTimeDifference` provides much more powerful configuration options
for rounding the span returned. Take this example, which produces a span with
many unit values:

```rust
use jiff::{civil::date, ToSpan, Unit};

fn main() -> anyhow::Result<()> {
    let zdt1 =
        date(2024, 5, 15).at(14, 30, 54, 0).in_tz("America/New_York")?;
    let zdt2 = date(2024, 5, 31)
        .at(19, 13, 27, 123_456_789)
        .in_tz("America/New_York")?;
    assert_eq!(
        zdt1.until((Unit::Day, &zdt2))?,
        16.days()
            .hours(4)
            .minutes(42)
            .seconds(33)
            .milliseconds(123)
            .microseconds(456)
            .nanoseconds(789)
            .fieldwise(),
    );

    Ok(())
}
```

This might be _too_ much precision to show to an end user. Instead, perhaps
we'd like to _round_ the span to the nearest half-hour. We can do this by
building a `ZonedDateTimeDifference` explicitly and configuring its options:

```rust
use jiff::{civil::date, RoundMode, ToSpan, Unit, ZonedDateTimeDifference};

fn main() -> anyhow::Result<()> {
    let zdt1 =
        date(2024, 5, 15).at(14, 30, 54, 0).in_tz("America/New_York")?;
    let zdt2 = date(2024, 5, 31)
        .at(19, 13, 27, 123_456_789)
        .in_tz("America/New_York")?;
    let difference = ZonedDateTimeDifference::new(&zdt2)
        .largest(Unit::Day)
        .smallest(Unit::Minute)
        .increment(30)
        .mode(RoundMode::HalfExpand);
    assert_eq!(
        zdt1.until(difference)?,
        16.days().hours(4).minutes(30).fieldwise(),
    );

    Ok(())
}
```

Note also that rounding a span takes time zone transitions like daylight saving
time into account. For example, let's take `2024-11-02 17:30` and
`2024-11-03 04:45` in New York. Because New York stopped daylight saving time
at 2AM on 2024-11-03, the amount of physical time between them is 12 hours and
15 minutes:

```rust
let zdt1 = date(2024, 11, 2).at(17, 30, 0, 0).in_tz("America/New_York")?;
let zdt2 = date(2024, 11, 3).at(4, 45, 0, 0).in_tz("America/New_York")?;
assert_eq!(zdt2 - zdt1, 12.hours().minutes(15).fieldwise());
```

Normally, if we rounded to the nearest day, this would round up to 1 day. But
since the midpoint of this particular day is actually 12 hours and 30 minutes,
this duration will round down to 0 days when we request it:

```rust
use jiff::{civil::date, RoundMode, ToSpan, Unit, ZonedDateTimeDifference};

fn main() -> anyhow::Result<()> {
    let zdt1 = date(2024, 11, 2).at(17, 30, 0, 0).in_tz("America/New_York")?;
    let zdt2 = date(2024, 11, 3).at(4, 45, 0, 0).in_tz("America/New_York")?;
    let difference = ZonedDateTimeDifference::new(&zdt2)
        .largest(Unit::Day)
        .smallest(Unit::Day)
        .mode(RoundMode::HalfExpand);
    assert_eq!(zdt1.until(difference)?, 0.days().fieldwise());

    Ok(())
}
```

Next we'll continue our discussion of rounding, but when applied to datetimes
themselves.

### Date Rounding

Date rounding in Jiff is limited to units of days or lower. Therefore, it's
only available on [`ZonedDateTime`], [`Timestamp`], [`civil::DateTime`],
[`civil::Time`] but not [`civil::Date`].

A common example for rounding a datetime is to reduce precision. For example,
a system clock might provide nanosecond precision, but maybe we only care about
the nearest half-hour:

```rust
use jiff::{civil::date, Unit};

fn main() -> anyhow::Result<()> {
    let zdt = date(2024, 11, 2)
        .at(17, 46, 20, 123_456_789)
        .in_tz("America/New_York")?;
    assert_eq!(
        zdt.to_string(),
        "2024-11-02T17:46:20.123456789-04:00[America/New_York]"
    );

    // Round to the nearest half hour.
    let rounded = zdt.round((Unit::Minute, 30))?;
    assert_eq!(
        rounded.to_string(),
        "2024-11-02T18:00:00-04:00[America/New_York]"
    );

    Ok(())
}
```

As with rounding spans, rounding a `ZonedDateTime` is aware of time zone
transitions:

```rust
let zdt = date(2024, 11, 3)
    .at(11, 45, 20, 123_456_789)
    .in_tz("America/New_York")?;

// Round to the nearest day.
let rounded = zdt.round(Unit::Day)?;
assert_eq!(
    rounded.to_string(),
    "2024-11-04T00:00:00-05:00[America/New_York]"
);
```

Notice that in the above example, our time of 11:45AM is _before_ noon, yet the
result still rounds _up_ to the next day. This is because the midpoint for this
day is 11:30AM since, by 11:30AM, 12.5 hours have already elapsed because of
the repeated 1AM hour in the move away from daylight saving time in New York.

### Other Calendars

Jiff itself doesn't support any calendars other than the proleptic Gregorian
calendar. One can instead use the [ICU4X] crate to work in non-Gregorian
calendars, and convert between datetime types using the [`jiff-icu`] crate:
(Use `cargo add icu jiff-icu` to make this example work.)

```rust
use icu::{
    calendar::{types::Month, Date as IcuDate},
    time::{DateTime as IcuDateTime, Time as IcuTime},
};
use jiff::civil::DateTime;
use jiff_icu::ConvertTryFrom as _;

fn main() -> anyhow::Result<()> {
    // Firstly, create an ICU datetime using the Hebrew calendar.
    let datetime = IcuDateTime {
        date: IcuDate::try_new_hebrew_v2(5785, Month::new(5), 4)?,
        time: IcuTime::try_new(17, 30, 0, 0)?,
    };

    // Secondly, convert it to a civil datetime.
    // This will automatically use ICU to convert
    // the date from the Hebrew calendar to the
    // Gregorian (or ISO) calendar.
    let zdt = DateTime::convert_try_from(datetime)?;
    assert_eq!(zdt.to_string(), "2025-02-02T17:30:00");
    Ok(())
}
```

The ICU4X API supports [many more calendars][ICU4X calendars].

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
and even parsing/formatting of datetime values. Note though that Jiff is not
and does not aim to be a 100% compatible implementation of the Temporal
specification. Jiff is meant to be a Rust ecosystem library first and foremost.

With that said, if you're familiar with Temporal and Rust, then it's likely
you'll find Jiff to be a relatively straightforward translation of Temporal's
API to Rust. This example shows how to use Temporal to round a calendar span
to the nearest number of days relative to a date:

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
* Jiff's datetime range only extends to +/- 9999, whereas Temporal's range is
larger.

It's easy to say that without Temporal, Jiff wouldn't exist in its current
form today.

## The Jiff 1.0 Commitment

Jiff is already used in production in many projects: uv, Cargo, Starship, fd,
Nushell, mise, Jujutsu, uutils/coreutils and even as the basis for a Temporal
implementation in AWS LLRT. It is time to call it 1.0.

Unless some significant defect is found in Jiff's API, my plan is to commit
to Jiff 1.0 indefinitely. This is important because the intent is for Jiff
to be an ecosystem building block that appears in the public API of other
libraries. This makes Jiff an attractive _public_ dependency, which means
semver incompatible releases are likely to cause significant pain in the
ecosystem. They will not be made lightly.

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
[Unix time]: https://en.wikipedia.org/wiki/Unix_time
[Gregorian calendar]: https://en.wikipedia.org/wiki/Gregorian_calendar
[falsehoods]: https://infiniteundo.com/post/25326999628/falsehoods-programmers-believe-about-time
[Civil time]: https://en.wikipedia.org/wiki/Civil_time
[UTC]: https://en.wikipedia.org/wiki/Coordinated_Universal_Time
[TAI]: https://en.wikipedia.org/wiki/International_Atomic_Time
[mean solar time]: https://en.wikipedia.org/wiki/Solar_time#Mean_solar_time
[Leap second]: https://en.wikipedia.org/wiki/Leap_second
[daylight saving time]: https://en.wikipedia.org/wiki/Daylight_saving_time
[`Span`]: https://docs.rs/jiff/1.*/jiff/struct.Span.html
[`SignedDuration`]: https://docs.rs/jiff/1.*/jiff/struct.SignedDuration.html
[`std::time::Duration`]: https://doc.rust-lang.org/nightly/std/time/struct.Duration.html
[`jiff-icu`]: https://docs.rs/jiff-icu
[ICU4X]: https://docs.rs/icu
[ICU4X calendars]: https://docs.rs/icu_calendar/latest/icu_calendar/
[RFC 2822]: https://www.rfc-editor.org/info/rfc2822/
[RFC 3339]: https://www.rfc-editor.org/info/rfc3339/
[RFC 9557]: https://www.rfc-editor.org/info/rfc9557/
[ISO 8601]: https://en.wikipedia.org/wiki/ISO_8601
[Temporal RFC 9557 / ISO 8601]: https://tc39.es/proposal-temporal/#sec-temporal-iso8601grammar
[`FromStr`]: https://doc.rust-lang.org/nightly/std/str/trait.FromStr.html
[`Display`]: https://doc.rust-lang.org/nightly/std/fmt/trait.Display.html
[IANA Time Zone Database]: https://www.iana.org/time-zones
[`jiff::fmt`]: https://docs.rs/jiff/1.*/jiff/fmt/index.html
[`jiff::fmt::friendly`]: https://docs.rs/jiff/1.*/jiff/fmt/friendly/index.html
[`jiff::fmt::strtime`]: https://docs.rs/jiff/1.*/jiff/fmt/strtime/index.html
[`serde`]: https://serde.rs/
[`Span::fieldwise`]: https://docs.rs/jiff/1.*/jiff/struct.Span.html#method.fieldwise
[`Span::compare`]: https://docs.rs/jiff/1.*/jiff/struct.Span.html#method.compare
[POSIX `strptime`]: https://pubs.opengroup.org/onlinepubs/009695399/functions/strptime.html
[POSIX `strftime`]: https://pubs.opengroup.org/onlinepubs/009695399/functions/strftime.html
[`ZonedDateTime::checked_add`]: https://docs.rs/jiff/1.*/jiff/struct.ZonedDateTime.html#method.checked_add
[`ZonedDateTimeArithmetic`]: https://docs.rs/jiff/1.*/jiff/struct.ZonedDateTimeArithmetic.html
[`Date::until`]: https://docs.rs/jiff/1.*/jiff/civil/struct.Date.html#method.until
[`ZonedDateTime::until`]: https://docs.rs/jiff/1.*/jiff/struct.ZonedDateTime.html#method.until
[`ZonedDateTimeDifference`]: https://docs.rs/jiff/1.*/jiff/struct.ZonedDateTimeDifference.html
[`std::time::Instant`]: https://doc.rust-lang.org/nightly/std/time/struct.Instant.html
[`std::time::SystemTime`]: https://doc.rust-lang.org/nightly/std/time/struct.SystemTime.html
