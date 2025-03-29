package solver

import (
	"math/rand"
	"time"

	"github.com/phdah/sql-tdg/internals/types"
)

type TimestampDomain struct {
	IntDomain
}

func NewTimestampDomain() *TimestampDomain {
	lower := 0
	upper := 4102358400
	return &TimestampDomain{
		IntDomain: IntDomain{
			Intervals: []types.Interval{{Min: lower, Max: upper}},
			TotalMin:  lower,
			TotalMax:  upper,
		},
	}
}

func (t TimestampDomain) RandomValue(rng *rand.Rand) any {
	raw := t.IntDomain.RandomValue(rng).(int)
	return time.Unix(int64(raw), 0)
}

// ToTimestamp parses a string in RFC3339 format (e.g. "2006-01-02T15:04:05Z")
// and returns the corresponding Unix timestamp as an int.
// Example input: "2013-06-17T00:00:00Z"
func ToTimestamp(timestamp string) int {
	t, _ := time.Parse(time.RFC3339, timestamp)
	return int(t.Unix())
}

// ToDate parses a date string in the format "yyyy-MM-dd" (e.g. "2013-06-17")
// and returns the corresponding Unix timestamp as an int (seconds since epoch).
// It ignores any parsing errors (assumes valid input).
func ToDate(timestamp string) int {
	t, _ := time.Parse("2006-01-02", timestamp)
	return int(t.Unix())
}

// FromInt converts an integer Unix timestamp (seconds since epoch)
// to a time.Time value in UTC.
func FromInt(timestamp int) time.Time {
	return time.Unix(int64(timestamp), 0)
}
