package solver

import (
	"fmt"
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

func (c IntEq) Apply(domain types.Domain) (types.Domain, error) {
	d := domain.(IntDomain)
	if is, err := intervalNotOverlaping(d, c.Value); is {
		return nil, fmt.Errorf("The condition is invalid, not in the established interval: %v", err)
	}
	d.Max = c.Value
	d.Min = c.Value
	return d, nil
}

func intervalNotOverlaping(domain types.Domain, value int) (bool, error) {

	return false, nil
}
