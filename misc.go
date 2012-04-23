package main

import (
	"time"
)

// pluralize is stupid. If its 0 or more than 1, just add an 's'.
// We'll deal with other cases as they arise.
func pluralize(n int, s string) string {
	if n == 1 {
		return s
	}
	return s + "s"
}

// formatTime is how time/date is always shown in my blog.
func formatTime(t time.Time) string {
	return t.Format("01/02/2006 at 3:04pm")
}

// timeFromNano gets a time.Time from the number of nanoseconds since
// the Unix Epoch.
func timeFromNano(n int64) time.Time {
	seconds := n / 1000000000
	nanosecs := n - seconds*1000000000

	return time.Unix(seconds, nanosecs)
}
