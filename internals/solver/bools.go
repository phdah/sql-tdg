package solver

import (
	"fmt"
	"math/rand"

	"github.com/phdah/sql-tdg/internals/types"
)

type BoolDomain struct {
	Condition bool
	HasBeenChanged bool
}

func (d *BoolDomain) GetTotalMin() any {
	return nil
}

func (d *BoolDomain) GetTotalMax() any {
	return nil
}

func NewBoolDomain() *BoolDomain {
	return &BoolDomain{
		Condition: true,
		HasBeenChanged: false,
	}
}

func (d *BoolDomain) SplitIntervals(splitValue any) error {
	return nil
}

func (d *BoolDomain) UpdateIntervals(newInterval types.Interval) error {
	return nil
}

func (d BoolDomain) RandomValue(rng *rand.Rand) (any, error) {
	return d.Condition, nil
}

type BoolTrue struct{ Value int }
type BoolFalse struct{ Value int }

func (c BoolTrue) Apply(domain types.Domain) error {
	boolDomain, ok := domain.(*BoolDomain)
	if !ok {
		return fmt.Errorf("expected BoolDomain, got %T", boolDomain)
	}
	if boolDomain.HasBeenChanged && !boolDomain.Condition {
		return fmt.Errorf("BoolDomain Condition has already been updated to false")
	}
	boolDomain.Condition = true
	boolDomain.HasBeenChanged = true
	return nil
}

func (c BoolFalse) Apply(domain types.Domain) error {
	boolDomain, ok := domain.(*BoolDomain)
	if !ok {
		return fmt.Errorf("expected BoolDomain, got %T", boolDomain)
	}
	if boolDomain.HasBeenChanged && boolDomain.Condition {
		return fmt.Errorf("BoolDomain Condition has already been updated to true")
	}
	boolDomain.Condition = false
	boolDomain.HasBeenChanged = true
	return nil
}
