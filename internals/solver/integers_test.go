package solver_test

import (
	"fmt"
	"testing"

	"github.com/phdah/sql-tdg/internals/solver"
	"github.com/phdah/sql-tdg/internals/types"
	"github.com/stretchr/testify/require"
)

func TestIntEq_Apply(t *testing.T) {
	tests := []struct {
		name string // description of this test case
		// Named input parameters for target function.
		domain     types.Domain
		want       types.Domain
		conditions []types.Constraints
	}{
		{
			name:   "only set one equal",
			domain: solver.NewIntDomain(),
			want: &solver.IntDomain{Intervals: []types.Interval{
				{Min: 3, Max: 3},
			},
				TotalMin: 3,
				TotalMax: 3},
			conditions: []types.Constraints{
				solver.IntEq{3},
			},
		},
		{
			name:   "set two equal",
			domain: solver.NewIntDomain(),
			want: &solver.IntDomain{Intervals: []types.Interval{
				{Min: 3, Max: 3},
			},
				TotalMin: 3,
				TotalMax: 3},
			conditions: []types.Constraints{
				solver.IntEq{3},
				solver.IntNEq{5},
			},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := require.New(t)
			var got types.Domain
			var err error
			for _, c := range tt.conditions {
				got, err = c.Apply(tt.domain)
				r.NoErrorf(err, fmt.Sprintf("Error was rasied: %v", err))
			}
			r.Equal(tt.want, got)
		})
	}
}
