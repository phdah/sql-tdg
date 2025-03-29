package solver

import (
	"fmt"
	"math/rand"

	"github.com/phdah/sql-tdg/internals/types"
	"github.com/phdah/sql-tdg/internals/utils"
)

type IntDomain struct {
	Intervals []types.Interval
	TotalMin  int
	TotalMax  int
}

func NewIntDomain() *IntDomain {
	lower := -1_000_000
	upper := 1_000_000
	return &IntDomain{Intervals: []types.Interval{
		{Min: lower, Max: upper},
	},
		TotalMin: lower,
		TotalMax: upper,
	}
}
func (d *IntDomain) SplitIntervals(splitValue int) error {
	var updated []types.Interval
	for _, interval := range d.Intervals {
		// If value is inside of interval, split it
		if interval.Min < splitValue && splitValue < interval.Max {
			updated = append(updated, types.Interval{Min: interval.Min, Max: splitValue - 1})
			updated = append(updated, types.Interval{Min: splitValue + 1, Max: interval.Max})
		} else {
			updated = append(updated, interval)
		}
	}

	d.Intervals = updated
	return nil
}

func (d *IntDomain) UpdateIntervals(newInterval types.Interval) error {
	// List with all updated, or not updated intervals
	var updated []types.Interval

	for _, interval := range d.Intervals {
		// No overlap — continue
		if interval.Min > newInterval.Max || interval.Max < newInterval.Min {
			continue
		}

		// Compute new min and max value
		minv := utils.Max(interval.Min, newInterval.Min)
		maxv := utils.Min(interval.Max, newInterval.Max)

		if minv > maxv {
			return fmt.Errorf("min value is larger than max value")
		}

		// Set new total min
		if minv > d.TotalMin {
			d.TotalMin = minv
		}
		// Set new total max
		if maxv < d.TotalMax {
			d.TotalMax = maxv
		}
		updated = append(updated, types.Interval{Min: minv, Max: maxv})
	}
	// If no intervals overlap - panic
	if len(updated) <= 0 {
		return fmt.Errorf("interval not allowed: %v", newInterval)
	}

	d.Intervals = updated
	return nil
}

func (d IntDomain) RandomValue() any {
	total := 0
	counts := make([]int, len(d.Intervals))

	for i, interval := range d.Intervals {
		count := interval.Max - interval.Min + 1
		counts[i] = count
		total += count
	}

	if total == 0 {
		return nil // or panic, depending on your use case
	}

	r := rand.Intn(total)
	for i, count := range counts {
		if r < count {
			return d.Intervals[i].Min + r
		}
		r -= count
	}

	return nil // unreachable
}

type IntEq struct{ Value int }
type IntNEq struct{ Value int }
type IntLt struct{ Value int }
type IntGt struct{ Value int }
type IntLte struct{ Value int }
type IntGte struct{ Value int }

func (c IntEq) Apply(domain types.Domain) (types.Domain, error) {
	d := domain.(*IntDomain)
	err := d.UpdateIntervals(types.Interval{Min: c.Value, Max: c.Value})
	return d, err
}

func (c IntNEq) Apply(domain types.Domain) (types.Domain, error) {
	d := domain.(*IntDomain)
	err := d.SplitIntervals(c.Value)
	return d, err
}

func (c IntLt) Apply(domain types.Domain) (types.Domain, error) {
	d := domain.(*IntDomain)
	err := d.UpdateIntervals(types.Interval{Min: d.TotalMin, Max: c.Value - 1})
	return d, err
}

func (c IntLte) Apply(domain types.Domain) (types.Domain, error) {
	d := domain.(*IntDomain)
	err := d.UpdateIntervals(types.Interval{Min: d.TotalMin, Max: c.Value})
	return d, err
}

func (c IntGt) Apply(domain types.Domain) (types.Domain, error) {
	d := domain.(*IntDomain)
	err := d.UpdateIntervals(types.Interval{Min: c.Value + 1, Max: d.TotalMax})
	return d, err
}

func (c IntGte) Apply(domain types.Domain) (types.Domain, error) {
	d := domain.(*IntDomain)
	err := d.UpdateIntervals(types.Interval{Min: c.Value, Max: d.TotalMax})
	return d, err
}
