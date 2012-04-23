package main

import (
	"time"
)

// pluralize is stupid.
func pluralize(n int, s string) string {
	if n == 1 {
		return s
	}
	return s + "s"
}

func formatTime(t time.Time) string {
	return t.Format("01/02/2006 at 3:04pm")
}

func timeFromNano(n int64) time.Time {
	seconds := n / 1000000000
	nanosecs := n - seconds*1000000000

	return time.Unix(seconds, nanosecs)
}
