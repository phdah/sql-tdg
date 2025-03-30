package types

import "math/rand"

type Type string

const (
	IntType       Type = "int"
	TimestampType Type = "timestamp"
	BoolType      Type = "bool"
	StringType    Type = "string"
)

type Column struct {
	Name        string
	Type        Type
	Constraints []Constraints
}

type Constraints interface {
	Apply(domain Domain) error
}

type Interval struct {
	Min int
	Max int
}

type Domain interface {
	GetTotalMin() any                        // Get intervals max
	GetTotalMax() any                        // Get intervals min
	RandomValue(rng *rand.Rand) (any, error) // Generate random value
	UpdateIntervals(interval Interval) error // Add another interval
	SplitIntervals(splitValue any) error     // Split intervals
}
