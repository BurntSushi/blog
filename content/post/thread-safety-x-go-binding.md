+++
date = "2012-04-21T20:52:00-05:00"
draft = false
title = "Adding Thread Safety to the X Go Binding"
author = "Andrew Gallant"
url = "thread-safety-x-go-binding"
+++

The [X Go Binding (XGB)](http://code.google.com/p/x-go-binding/) is a low level
library that provides an API to interact with running X servers. One can only
communicate with an X server by sending data over a network connection;
protocol requests, replies and errors need to be perfectly constructed down to
the last byte. Xlib did precisely this, and then some. As a result, Xlib became
huge and difficult to maintain.

In recent years, the [XCB project](http://xcb.freedesktop.org/) made things a
bit more civilized by generating C code from [XML
files](http://cgit.freedesktop.org/xcb/proto/tree/src) describing the X client
protocol using Python. While [the Python to generate said
code](http://cgit.freedesktop.org/xcb/libxcb/tree/src/c_client.py) is no walk
in the park, it is typically preferred to the alternative: keeping the X core
protocol up to date along with any number of extensions that exist as well.
(There are other benefits to XCB, like easier asynchronicity, but that is
beyond the scope of this post.)

XGB proceeds in a similar vain; [Python is used to generate Go
code](http://code.google.com/r/jamslam-x-go-binding/source/browse/xgb/go_client.py)
that provides an API to interact with the X protocol. Unlike its sister project
XCB, it is not thread safe. In particular, if X requests are made in parallel,
the best case scenario is a jumbled request or reply and the worst case
scenario is an X server crash. Parallel requests can be particularly useful
when images are being sent to the X server to be painted on the screen; other
useful work could be done in the interim.

<!--more-->

For example, in [Wingo](https://github.com/BurntSushi/wingo) (a window manager
written purely in Go; still in development), it would be great to do something
like this when first managing a client:

``` go
func manage(windowId xgb.Id) {
    go func() {
        // load window icon images
        // and turn them into X pixmaps
    }()

    // the rest of the code to manage a client
}
```

Assuming `GOMAXPROCS` has been set appropriately, this would allow Wingo to show
and map a window without regard to the time required to prep images associated
with each client.
(i.e., icons, images containing the title of the window, alt-tab cylcing icons,
etc.)
Such parallelism is particularly useful when the user has configured Wingo to
use large images---which noticeably results in lag when first managing a
window.
Without thread safety in XGB, this sort of parallelism is impossible.
Since drawing images to X windows is a common task, parallelism can be
particularly useful.
Thus, thread safety in XGB---being the only barrier to this sort of
parallelism---is certainly desirable.

This is a perfect opportunity for [Go](http://golang.org) to shine.
But before we get into the juicy tidbits, I must discuss a few low-level and
nasty details of X.

As said previously, we communicate with X over a network connection.
As a client, we send requests and we read replies, events and errors.
In particular, replies, events and errors all come to us on the same wire---XGB
must deduce which kind of thing its reading by looking at the value of the
first byte of each 32 byte block.
(Sometimes replies can be longer than 32 bytes, but we can safely skip over
that detail for this post.)

On a conceptual level, events are inherently separate from replies and errors.
In particular, when a request expects a response (not all requests do!), it
will either get a reply *or* an error from X.
Thus, when issuing a request that expects a response, we are implicitly
creating a contract with the X server that we'll receive something
corresponding to that request.
We will revisit this **response contract** later; remember it!

(In this post, I'll be focusing on the thread safety of receiving *responses*.
Some amount of work also had to be done to ensure thread safe writing and
reading of events---among other things.
But the thread safety of receiving responses is much more interesting.)

You may be wondering: how do requests and responses match up?
Both X and XGB keep track of how many requests have been sent.
Each request we send, therefore, has a unique serial identifier associated with
it.
This identifier is also known as a cookie---and it's included in every single
reply and error sent to us from the X server.
Therefore, whenever we send a request that expects a response, we need some way
to store the cookie so we know when we've received the response (which will
either be a reply or an error).
Naively, this could be a simple map from cookie identifiers to responses. Here
are the types:

``` go
type Cookies map[uint16]*Response
type Response struct {
    reply []byte
    err error
}
```

The types say that we have a map of cookie identifiers (unsigned 16-bit
integers) to responses---where responses are either a reply (some slice of
bytes) or an error. We could then populate this map by reading from our X
connection with something like (excuse some pseudo code to keep it brief):

``` go
cookies := make(Cookies)

go func() {
    for {
        io.ReadFull(xConn, responseBytes)
        cookieId := getCookieFromBytes(responseBytes)

        if _, ok := cookies[cookieId]; ok {
            if responseBytes is reply {
                cookies[cookieId] = &Response{reply: responseBytes}
            } else if responseBytes is error {
                cookies[cookieId] = &Response{err: errorFromBytes(responseBytes)}
            } else {
                panic("unreachable")
            }
        } else {
            panic("Got unexpected response")
        }
    }
}()
```

But now what? If we're waiting for a particular reply (i.e., with some
particular cookie identifier), we could try something like:

``` go
func WaitForReply(cookieId uint16) *Response {
    for {
        if response, ok := cookies[cookieId]; ok {
            return response
        }
        time.Sleep(???)
    }
}
```

But how much time should we wait between checks?
If it's too short, we'll end up spinning and if it's too long we'll be blocking
when we should be handling a response.

The underlying problem here is that replies may not come in the order that we
need them.
We can't wait on a single channel for responses to come in, because we
may be waiting for more than one reply and we can't be sure of the order
they'll arrive in.

Another way to think about this problem is in terms of the **response
contract** I mentioned earlier.
The response contract is quite specific: whenever a request is sent that
expects a response (we know which requests expect a response ahead of time), it
*must* get either a reply or an error from the X server.

This is a perfect situation for goroutines and channels.
Instead of thinking about the cookie as some identifier yielding a response, we
can think about a cookie as an identifier with a "promise" it will return
either a reply or an error in the future.
That "promise" can be represented as a pair of channels: one channel gets a
reply and the other gets an error.
Let's revisit our types:

``` go
type Cookies map[uint16]*Cookie
type Cookie struct {
    replyChan make(chan []byte, 1)
    errChan make(chan error, 1)
}
```

So that `replyChan` and `errChan` are the pieces that will fulfill the cookie's
promise (they are buffered so they don't block the X read loop).
The promise is fullfilled in the code that reads from the X connection.
Instead of creating a response that has either a reply or an error, we can use
the cookie's channels to send either a reply or an error.
Only two lines need changing:

``` go
cookies := make(Cookies)

go func() {
    for {
        io.ReadFull(xConn, responseBytes)
        cookieId := getCookieFromBytes(responseBytes)

        if cookie, ok := cookies[cookieId]; ok {
            if responseBytes is reply {
-->             cookie.replyChan <- responseBytes
            } else if responseBytes is error {
-->             cookie.errChan <- errorFromBytes(responseBytes)
            } else {
                panic("unreachable")
            }
        } else {
            panic("Got unexpected response")
        }
    }
}()
```

Our WaitForReply code also becomes much better, and will always do just the
right amount of blocking:

``` go
func WaitForReply(cookie *Cookie) ([]byte, error) {
    select {
        case reply := <-cookie.replyChan:
            return reply, nil
        case err := <-cookie.errChan:
            return nil, err
    }
    panic("unreachable")
}
```

So that we now have thread safe---and parallel---responses.

You can find my work in a clone of XGB called
[jamslam-x-go-binding](http://code.google.com/r/jamslam-x-go-binding).
In addition to thead safety, several bugs have been fixed (most notably with
using ChangeProperty on 32 bit values) and support for the Xinerama extension
has been added. Support for other extensions is on the roadmap.

Any comments or criticisms on my approach are greatly appreciated.

