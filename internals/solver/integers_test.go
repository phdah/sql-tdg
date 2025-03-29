package solver_test

import (
	"fmt"
	"testing"

	"github.com/phdah/sql-tdg/internals/solver"
	"github.com/phdah/sql-tdg/internals/types"
	"github.com/stretchr/testify/require"
)

func TestInt_Single_Apply(t *testing.T) {
	tests := []struct {
		name string // description of this test case
		// Named input parameters for target function.
		domain     types.Domain
		want       types.Domain
		wantErr    error
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
			wantErr: nil,
		},
		{
			name:   "only set one not equal",
			domain: solver.NewIntDomain(),
			want: &solver.IntDomain{Intervals: []types.Interval{
				{Min: -1_000_000, Max: 2},
				{Min: 4, Max: 1_000_000},
			},
				TotalMin: -1_000_000,
				TotalMax: 1_000_000},
			conditions: []types.Constraints{
				solver.IntNEq{3},
			},
			wantErr: nil,
		},
		{
			name:   "only set one less than",
			domain: solver.NewIntDomain(),
			want: &solver.IntDomain{Intervals: []types.Interval{
				{Min: -1_000_000, Max: 2},
			},
				TotalMin: -1_000_000,
				TotalMax: 2},
			conditions: []types.Constraints{
				solver.IntLt{3},
			},
			wantErr: nil,
		},
		{
			name:   "only set one less or equal to",
			domain: solver.NewIntDomain(),
			want: &solver.IntDomain{Intervals: []types.Interval{
				{Min: -1_000_000, Max: 3},
			},
				TotalMin: -1_000_000,
				TotalMax: 3},
			conditions: []types.Constraints{
				solver.IntLte{3},
			},
			wantErr: nil,
		},
		{
			name:   "only set one greater than",
			domain: solver.NewIntDomain(),
			want: &solver.IntDomain{Intervals: []types.Interval{
				{Min: 4, Max: 1_000_000},
			},
				TotalMin: 4,
				TotalMax: 1_000_000},
			conditions: []types.Constraints{
				solver.IntGt{3},
			},
			wantErr: nil,
		},
		{
			name:   "only set one greater or equal to",
			domain: solver.NewIntDomain(),
			want: &solver.IntDomain{Intervals: []types.Interval{
				{Min: 3, Max: 1_000_000},
			},
				TotalMin: 3,
				TotalMax: 1_000_000},
			conditions: []types.Constraints{
				solver.IntGte{3},
			},
			wantErr: nil,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := require.New(t)
			var got types.Domain
			var err error
			for _, c := range tt.conditions {
				got, err = c.Apply(tt.domain)
				if tt.wantErr != nil && err != nil {
					r.Error(err)
					r.Contains(tt.wantErr.Error(), err.Error())
					return
				}
				r.NoErrorf(err, fmt.Sprintf("Error was rasied: %v", err))
			}
			r.Equal(tt.want, got)
		})
	}
}

func TestInt_Multi_Apply(t *testing.T) {
	tests := []struct {
		name string // description of this test case
		// Named input parameters for target function.
		domain     types.Domain
		want       types.Domain
		wantErr    error
		conditions []types.Constraints
	}{
		{
			name:   "set one of each, all applied",
			domain: solver.NewIntDomain(),
			want: &solver.IntDomain{Intervals: []types.Interval{
				{Min: 3, Max: 3},
			},
				TotalMin: 3,
				TotalMax: 3},
			conditions: []types.Constraints{
				solver.IntEq{3}, // Since equal is set, this should be the final one
				solver.IntGt{-10},
				solver.IntGte{0},
				solver.IntLt{200},
				solver.IntLte{150},
				solver.IntNEq{100},
			},
			wantErr: nil,
		},
		{
			name:   "not allowed intervals, panic",
			domain: solver.NewIntDomain(),
			want: &solver.IntDomain{Intervals: []types.Interval{
				{Min: 3, Max: 3},
			},
				TotalMin: 3,
				TotalMax: 3},
			conditions: []types.Constraints{
				solver.IntNEq{5},
				solver.IntEq{5},
			},
			wantErr: fmt.Errorf("interval not allowed: {5 5}"),
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := require.New(t)
			var got types.Domain
			var err error
			for _, c := range tt.conditions {
				got, err = c.Apply(tt.domain)
				if tt.wantErr != nil && err != nil {
					r.Error(err)
					r.Contains(tt.wantErr.Error(), err.Error())
					return
				}
				r.NoErrorf(err, fmt.Sprintf("Error was rasied: %v", err))
			}
			r.Equal(tt.want, got)
		})
	}
}
