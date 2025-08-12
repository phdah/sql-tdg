package solver

import (
	"fmt"
	"math/rand"
	"time"
	"strings"

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

func (t TimestampDomain) RandomValue(rng *rand.Rand) (any, error) {
	val, err := t.IntDomain.RandomValue(rng)
	raw, ok := val.(int)
	if !ok {
		return time.Time{}, fmt.Errorf("expected int, got %T", val)
	}
	return time.Unix(int64(raw), 0), err
}

// ToTimestamp parses a string in RFC3339 format (e.g. "2006-01-02T15:04:05Z")
// and returns the corresponding Unix timestamp as an int.
// Example input: "2013-06-17T00:00:00Z"
func ToTimestamp(timestamp string) int {
	// Trim both single and double quotes from the string.
	// This handles `"2013-06-17"` and `'2013-06-17'`.
	timestamp = strings.Trim(timestamp, `"'`)
	t, _ := time.Parse(time.RFC3339, timestamp)
	return int(t.Unix())
}

// ToDate parses a date string in the format "yyyy-MM-dd" (e.g. "2013-06-17")
// and returns the corresponding Unix timestamp as an int (seconds since epoch).
// It ignores any parsing errors (assumes valid input).
func ToDate(timestamp string) int {
	// Trim both single and double quotes from the string.
	// This handles `"2013-06-17"` and `'2013-06-17'`.
	timestamp = strings.Trim(timestamp, `"'`)
	t, _ := time.Parse("2006-01-02", timestamp)
	return int(t.Unix())
}

// ParseTime dynamically determines the format of a date or timestamp string
// and returns the corresponding Unix timestamp.
func ParseTime(timestamp string) (int, error) {
	// 1. Trim quotes from the string
	timestamp = strings.Trim(timestamp, `"'`)

	// 2. Try the most specific format first: RFC3339
	t, err := time.Parse(time.RFC3339, timestamp)
	if err == nil {
		return int(t.Unix()), nil
	}

	// 3. If RFC3339 fails, try the date-only format: "2006-01-02"
	t, err = time.Parse("2006-01-02", timestamp)
	if err == nil {
		return int(t.Unix()), nil
	}

	// 4. If all known formats fail, return an error
	return 0, fmt.Errorf("could not parse timestamp or date: %s", timestamp)
}

// FromInt converts an integer Unix timestamp (seconds since epoch)
// to a time.Time value in UTC.
func FromInt(timestamp int) time.Time {
	return time.Unix(int64(timestamp), 0)
}
