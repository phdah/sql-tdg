package solver

import (
	"fmt"
	"math/rand"
	"strings"
	"time"

	"github.com/phdah/sql-tdg/internals/types"
)

type TimestampDomain struct {
	IntDomain
}

func NewTimestampDomain() *TimestampDomain {
	lower := int32(0)
	upper := int32(2147483647)
	return &TimestampDomain{
		IntDomain: IntDomain{
			Intervals: []types.Interval{{Min: int(lower), Max: int(upper)}},
			TotalMin:  lower,
			TotalMax:  upper,
		},
	}
}

func (t TimestampDomain) RandomValue(rng *rand.Rand) (any, error) {
	val, err := t.IntDomain.RandomValue(rng)
	raw, ok := val.(int32)
	if !ok {
		return time.Time{}, fmt.Errorf("expected int32, got %T", val)
	}
	return time.Unix(int64(raw), 0), err
}

// ToTimestamp parses a string in RFC3339 format and returns the
// corresponding Unix timestamp as an int32.
func ToTimestamp(timestamp string) int32 {
	timestamp = strings.Trim(timestamp, `"'`)
	t, _ := time.Parse(time.RFC3339, timestamp)
	return int32(t.Unix())
}

// ToDate parses a date string and returns the corresponding
// Unix timestamp as an int32.
func ToDate(timestamp string) int32 {
	timestamp = strings.Trim(timestamp, `"'`)
	t, _ := time.Parse("2006-01-02", timestamp)
	return int32(t.Unix())
}

// ParseTime dynamically determines the format of a date or timestamp string
// and returns the corresponding Unix timestamp as an int32.
func ParseTime(timestamp string) (int32, error) {
	timestamp = strings.Trim(timestamp, `"'`)

	t, err := time.Parse(time.RFC3339, timestamp)
	if err == nil {
		return int32(t.Unix()), nil
	}

	t, err = time.Parse("2006-01-02", timestamp)
	if err == nil {
		return int32(t.Unix()), nil
	}

	return 0, fmt.Errorf("could not parse timestamp or date: %s", timestamp)
}

// FromInt converts an int32 Unix timestamp to a time.Time value in UTC.
func FromInt(timestamp int32) time.Time {
	return time.Unix(int64(timestamp), 0).UTC()
}
