package solver_test

import (
	"fmt"
	"testing"
	"time"

	"github.com/phdah/sql-tdg/internals/solver"
	"github.com/phdah/sql-tdg/internals/types"
	"github.com/stretchr/testify/require"
)

func TestTimestamp_Single_Apply(t *testing.T) {
	tests := []struct {
		name       string
		domain     types.Domain
		want       types.Domain
		wantErr    error
		conditions []types.Constraints
	}{
		{
			name:   "only set one equal",
			domain: solver.NewTimestampDomain(),
			want: &solver.TimestampDomain{
				IntDomain: solver.IntDomain{
					Intervals: []types.Interval{{Min: 3, Max: 3}},
					TotalMin:  3,
					TotalMax:  3,
				},
			},
			conditions: []types.Constraints{
				solver.IntEq{Value: 3},
			},
			wantErr: nil,
		},
		{
			name:   "only set one not equal",
			domain: solver.NewTimestampDomain(),
			want: &solver.TimestampDomain{
				IntDomain: solver.IntDomain{
					Intervals: []types.Interval{
						{Min: 0, Max: 2},
						// Max value is capped at int32 max
						{Min: 4, Max: 2147483647},
					},
					TotalMin: 0,
					// Max value is capped at int32 max
					TotalMax: 2147483647,
				},
			},
			conditions: []types.Constraints{
				solver.IntNEq{Value: 3},
			},
			wantErr: nil,
		},
		{
			name:   "only set one less than",
			domain: solver.NewTimestampDomain(),
			want: &solver.TimestampDomain{
				IntDomain: solver.IntDomain{
					Intervals: []types.Interval{
						{Min: 0, Max: 2},
					},
					TotalMin: 0,
					TotalMax: 2,
				},
			},
			conditions: []types.Constraints{
				solver.IntLt{Value: 3},
			},
			wantErr: nil,
		},
		{
			name:   "only set one less or equal to",
			domain: solver.NewTimestampDomain(),
			want: &solver.TimestampDomain{
				IntDomain: solver.IntDomain{
					Intervals: []types.Interval{
						{Min: 0, Max: 3},
					},
					TotalMin: 0,
					TotalMax: 3,
				},
			},
			conditions: []types.Constraints{
				solver.IntLte{Value: 3},
			},
			wantErr: nil,
		},
		{
			name:   "only set one greater than",
			domain: solver.NewTimestampDomain(),
			want: &solver.TimestampDomain{
				IntDomain: solver.IntDomain{
					Intervals: []types.Interval{
						{Min: 4, Max: 2147483647},
					},
					TotalMin: 4,
					TotalMax: 2147483647,
				},
			},
			conditions: []types.Constraints{
				solver.IntGt{Value: 3},
			},
			wantErr: nil,
		},
		{
			name:   "only set one greater or equal to",
			domain: solver.NewTimestampDomain(),
			want: &solver.TimestampDomain{
				IntDomain: solver.IntDomain{
					Intervals: []types.Interval{
						{Min: 3, Max: 2147483647},
					},
					TotalMin: 3,
					TotalMax: 2147483647,
				},
			},
			conditions: []types.Constraints{
				solver.IntGte{Value: 3},
			},
			wantErr: nil,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := require.New(t)
			var err error
			for _, c := range tt.conditions {
				err = c.Apply(tt.domain)
				if tt.wantErr != nil && err != nil {
					r.Error(err)
					r.Contains(tt.wantErr.Error(), err.Error())
					return
				}
				r.NoErrorf(err, fmt.Sprintf("Error was rasied: %v", err))
			}
			r.Equal(tt.want, tt.domain)
		})
	}
}

func TestTimestamp_Multi_Apply(t *testing.T) {
	tests := []struct {
		name       string
		domain     types.Domain
		want       types.Domain
		wantErr    error
		conditions []types.Constraints
	}{
		{
			name:   "set one of each, all applied",
			domain: solver.NewTimestampDomain(),
			want: &solver.TimestampDomain{
				IntDomain: solver.IntDomain{
					Intervals: []types.Interval{
						{Min: 3, Max: 3},
					},
					TotalMin: 3,
					TotalMax: 3,
				},
			},
			conditions: []types.Constraints{
				solver.IntEq{Value: 3},
				solver.IntGt{Value: -10},
				solver.IntGte{Value: 0},
				solver.IntLt{Value: 200},
				solver.IntLte{Value: 150},
				solver.IntNEq{Value: 100},
			},
			wantErr: nil,
		},
		{
			name:   "not allowed intervals, panic",
			domain: solver.NewTimestampDomain(),
			want: &solver.TimestampDomain{
				IntDomain: solver.IntDomain{
					Intervals: []types.Interval{
						{Min: 5, Max: 5},
					},
					TotalMin: 5,
					TotalMax: 5,
				},
			},
			conditions: []types.Constraints{
				solver.IntNEq{Value: 5},
				solver.IntEq{Value: 5},
			},
			wantErr: fmt.Errorf("interval not allowed: {5 5}"),
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := require.New(t)
			var err error
			for _, c := range tt.conditions {
				err = c.Apply(tt.domain)
				if tt.wantErr != nil && err != nil {
					r.Error(err)
					r.Contains(tt.wantErr.Error(), err.Error())
					return
				}
				r.NoErrorf(err, fmt.Sprintf("Error was rasied: %v", err))
			}
			r.Equal(tt.want, tt.domain)
		})
	}
}

func TestToDate(t *testing.T) {
	tests := []struct {
		name string
		date string
		want int32
	}{
		{
			name: "working string to timestamp",
			date: "2013-06-17",
			want: 1371427200,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := require.New(t)
			got := solver.ToDate(tt.date)
			r.Equal(tt.want, got)
		})
	}
}

func TestToTimestamp(t *testing.T) {
	tests := []struct {
		name      string
		timestamp string
		want      int32
	}{
		{
			name:      "working string to timestamp",
			timestamp: "2013-06-17T12:25:04Z",
			want:      1371471904,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := require.New(t)
			got := solver.ToTimestamp(tt.timestamp)
			r.Equal(tt.want, got)
		})
	}
}

func TestFromInt(t *testing.T) {
	tests := []struct {
		name      string
		timestamp int32
		want      time.Time
	}{
		{
			name:      "date with explicit int32",
			timestamp: 1371427200,
			want:      time.Date(2013, time.June, 17, 0, 0, 0, 0, time.UTC),
		},
		{
			name:      "timestamp with explicit int32",
			timestamp: 1371471904,
			want:      time.Date(2013, time.June, 17, 12, 25, 4, 0, time.UTC),
		},
		{
			name:      "date with explicit int32",
			timestamp: solver.ToDate("2013-06-17"),
			want:      time.Date(2013, time.June, 17, 0, 0, 0, 0, time.UTC),
		},
		{
			name:      "timestamp with explicit int32",
			timestamp: solver.ToTimestamp("2013-06-17T12:25:04Z"),
			want:      time.Date(2013, time.June, 17, 12, 25, 4, 0, time.UTC),
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := require.New(t)
			got := solver.FromInt(tt.timestamp)
			r.Equal(tt.want, got)
		})
	}
}
