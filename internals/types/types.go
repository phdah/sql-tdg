package types

type Type string

const (
	IntType    Type = "int"
	BoolType   Type = "bool"
	StringType Type = "string"
)

type Column struct {
	Name        string
	Type        Type
	Constraints []Constraints
}

type Constraints interface {
	Apply(domain Domain) (Domain, error)
}

type Domain interface {
	RandomValue() any // Generate random value
}
