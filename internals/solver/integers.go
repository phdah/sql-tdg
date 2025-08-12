package solver

import (
	"fmt"
	"math/rand"

	"github.com/phdah/sql-tdg/internals/types"
	"github.com/phdah/sql-tdg/internals/utils"
)

type IntDomain struct {
	Intervals []types.Interval
	TotalMin  int32
	TotalMax  int32
}

func (d *IntDomain) GetTotalMin() any {
	return d.TotalMin
}

func (d *IntDomain) GetTotalMax() any {
	return d.TotalMax
}

func NewIntDomain() *IntDomain {
	lower := int32(-1_000_000)
	upper := int32(1_000_000)
	return &IntDomain{Intervals: []types.Interval{
		{Min: int(lower), Max: int(upper)},
	},
		TotalMin: lower,
		TotalMax: upper,
	}
}

func (d *IntDomain) SplitIntervals(splitValue any) error {
	splitValueInt, ok := splitValue.(int32)
	if !ok {
		return fmt.Errorf("expected int32, got %T", splitValue)
	}
	var updated []types.Interval
	for _, interval := range d.Intervals {
		// If value is inside of interval, split it
		if int32(interval.Min) < splitValueInt && splitValueInt < int32(interval.Max) {
			updated = append(updated, types.Interval{
				Min: interval.Min, Max: int(splitValueInt - 1),
			})
			updated = append(updated, types.Interval{
				Min: int(splitValueInt + 1), Max: interval.Max,
			})
		} else if int32(interval.Min) == splitValueInt {
			updated = append(updated, types.Interval{
				Min: int(splitValueInt + 1), Max: interval.Max,
			})
		} else if int32(interval.Max) == splitValueInt {
			updated = append(updated, types.Interval{
				Min: interval.Min, Max: int(splitValueInt - 1),
			})
		} else {
			updated = append(updated, interval)
		}
	}

	d.Intervals = updated
	return nil
}

func (d *IntDomain) UpdateIntervals(newInterval types.Interval) error {
	var updated []types.Interval

	for _, interval := range d.Intervals {
		if interval.Min > newInterval.Max || interval.Max < newInterval.Min {
			continue
		}

		minv := utils.Max(interval.Min, newInterval.Min)
		maxv := utils.Min(interval.Max, newInterval.Max)

		if minv > maxv {
			return fmt.Errorf("min value is larger than max value")
		}

		if int32(minv) > d.TotalMin {
			d.TotalMin = int32(minv)
		}
		if int32(maxv) < d.TotalMax {
			d.TotalMax = int32(maxv)
		}
		updated = append(updated, types.Interval{Min: minv, Max: maxv})
	}

	if len(updated) <= 0 {
		return fmt.Errorf("interval not allowed: %v", newInterval)
	}

	d.Intervals = updated
	return nil
}

func (d IntDomain) RandomValue(rng *rand.Rand) (any, error) {
	total := 0
	counts := make([]int, len(d.Intervals))

	for i, interval := range d.Intervals {
		count := interval.Max - interval.Min + 1
		counts[i] = count
		total += count
	}

	if total == 0 {
		return nil, fmt.Errorf("no values to generate")
	}

	r := rng.Intn(total)
	for i, count := range counts {
		if r < count {
			return int32(d.Intervals[i].Min + r), nil
		}
		r -= count
	}

	return nil, nil
}

// Constraint types updated to use int32
type IntEq struct{ Value int32 }
type IntNEq struct{ Value int32 }
type IntLt struct{ Value int32 }
type IntGt struct{ Value int32 }
type IntLte struct{ Value int32 }
type IntGte struct{ Value int32 }

// Updated Apply methods to handle int32
func (c IntEq) Apply(domain types.Domain) error {
	err := domain.UpdateIntervals(types.Interval{Min: int(c.Value), Max: int(c.Value)})
	return err
}

func (c IntNEq) Apply(domain types.Domain) error {
	err := domain.SplitIntervals(c.Value)
	return err
}

func (c IntLt) Apply(domain types.Domain) error {
	err := domain.UpdateIntervals(types.Interval{
		Min: int(domain.GetTotalMin().(int32)), Max: int(c.Value - 1),
	})
	return err
}

func (c IntLte) Apply(domain types.Domain) error {
	err := domain.UpdateIntervals(types.Interval{
		Min: int(domain.GetTotalMin().(int32)), Max: int(c.Value),
	})
	return err
}

func (c IntGt) Apply(domain types.Domain) error {
	err := domain.UpdateIntervals(types.Interval{
		Min: int(c.Value + 1), Max: int(domain.GetTotalMax().(int32)),
	})
	return err
}

func (c IntGte) Apply(domain types.Domain) error {
	err := domain.UpdateIntervals(types.Interval{
		Min: int(c.Value), Max: int(domain.GetTotalMax().(int32)),
	})
	return err
}
