package solver

import (
	"math/rand"

	"github.com/phdah/sql-tdg/internals/types"
)

type IntDomain struct {
	Min int
	Max int
}

func (d IntDomain) RandomValue() any {
	random := rand.Intn(d.Max - d.Min + 1)
	return random + d.Min
}

type IntEq struct{ Value int }
type IntNEq struct{ Value int }
type IntLt struct{ Value int }
type IntGt struct{ Value int }
type IntLte struct{ Value int }
type IntGte struct{ Value int }

func (c IntEq) Apply(domain types.Domain) types.Domain {
	d := domain.(IntDomain)
	d.Max = c.Value
	d.Min = c.Value
	return d
}
