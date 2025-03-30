package solver_test

import (
	"testing"
	"time"

	"github.com/stretchr/testify/require"

	"github.com/phdah/sql-tdg/internals/solver"
	"github.com/phdah/sql-tdg/internals/table"
	"github.com/phdah/sql-tdg/internals/types"
)

func TestIntGenerator_Generate(t *testing.T) {
	seed := int64(42)
	tests := []struct {
		name          string
		table         *table.Table
		expected      any
		expectedError error
	}{
		{
			name: "test with one column single condition",
			table: table.NewTable([]types.Column{
				{
					Name: "col_a",
					Type: types.IntType,
					Constraints: []types.Constraints{
						solver.IntEq{10}, // Column should be all equal to 10
					},
				},
			}, 12),
			expected: map[string][]int{
				"col_a": {10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10},
			},
			expectedError: nil,
		},
		{
			name: "test with one column multi condition",
			table: table.NewTable([]types.Column{
				{
					Name: "col_a",
					Type: types.IntType,
					Constraints: []types.Constraints{
						solver.IntNEq{10},
						solver.IntGt{3},
						solver.IntLt{100},
					},
				},
			}, 12),
			expected: map[string][]int{
				"col_a": {28, 42, 48, 57, 58, 64, 71, 73, 76, 84, 90, 95},
			},
			expectedError: nil,
		},
		{
			name: "test with two columns",
			table: table.NewTable([]types.Column{
				{
					Name: "col_a",
					Type: types.IntType,
					Constraints: []types.Constraints{
						solver.IntEq{10}, // Column should be all equal to 10
					},
				},
				{
					Name: "col_b",
					Type: types.IntType,
					Constraints: []types.Constraints{
						solver.IntEq{3}, // Column should be all equal to 3
					},
				},
			}, 12),
			expected: map[string][]int{
				"col_a": {10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10},
				"col_b": {3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3},
			},
			expectedError: nil,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := require.New(t)
			var g solver.Generator
			g.Generate(tt.table, seed)
			tt.table.SortInts()
			r.Equal(tt.expected, tt.table.Ints)
		})
	}
}

func TestTimestampGenerator_Generate(t *testing.T) {
	seed := int64(42)
	date_1 := solver.FromInt(solver.ToDate("2013-06-17"))
	date_2 := solver.FromInt(solver.ToTimestamp("2013-06-17T15:21:00Z"))
	tests := []struct {
		name          string
		table         *table.Table
		expected      any
		expectedError error
	}{
		{
			name: "test with one column single condition",
			table: table.NewTable([]types.Column{
				{
					Name: "col_a",
					Type: types.TimestampType,
					Constraints: []types.Constraints{
						solver.IntEq{solver.ToDate("2013-06-17")},
					},
				},
			}, 12),
			expected: map[string][]time.Time{
				"col_a": {
					date_1, date_1, date_1, date_1, date_1, date_1,
					date_1, date_1, date_1, date_1, date_1, date_1,
				},
			},
			expectedError: nil,
		},
		{
			name: "test with one column multi condition",
			table: table.NewTable([]types.Column{
				{
					Name: "col_a",
					Type: types.TimestampType,
					Constraints: []types.Constraints{
						solver.IntNEq{solver.ToTimestamp("2013-06-17T15:21:00Z")},
						solver.IntGt{solver.ToTimestamp("2013-06-17T15:10:00Z")},
						solver.IntLt{solver.ToTimestamp("2013-06-17T15:45:00Z")},
					},
				},
			}, 12),
			expected: map[string][]time.Time{
				"col_a": {
					solver.FromInt(solver.ToTimestamp("2013-06-17T15:23:25Z")),
					solver.FromInt(solver.ToTimestamp("2013-06-17T15:24:40Z")),
					solver.FromInt(solver.ToTimestamp("2013-06-17T15:26:36Z")),
					solver.FromInt(solver.ToTimestamp("2013-06-17T15:28:42Z")),
					solver.FromInt(solver.ToTimestamp("2013-06-17T15:30:22Z")),
					solver.FromInt(solver.ToTimestamp("2013-06-17T15:31:20Z")),
					solver.FromInt(solver.ToTimestamp("2013-06-17T15:32:23Z")),
					solver.FromInt(solver.ToTimestamp("2013-06-17T15:34:23Z")),
					solver.FromInt(solver.ToTimestamp("2013-06-17T15:36:02Z")),
					solver.FromInt(solver.ToTimestamp("2013-06-17T15:37:10Z")),
					solver.FromInt(solver.ToTimestamp("2013-06-17T15:40:12Z")),
					solver.FromInt(solver.ToTimestamp("2013-06-17T15:42:31Z")),
				},
			},
			expectedError: nil,
		},
		{
			name: "test with two columns",
			table: table.NewTable([]types.Column{
				{
					Name: "col_a",
					Type: types.TimestampType,
					Constraints: []types.Constraints{
						solver.IntEq{solver.ToDate("2013-06-17")},
					},
				},
				{
					Name: "col_b",
					Type: types.TimestampType,
					Constraints: []types.Constraints{
						solver.IntEq{solver.ToTimestamp("2013-06-17T15:21:00Z")},
					},
				},
			}, 12),
			expected: map[string][]time.Time{
				"col_a": {
					date_1, date_1, date_1, date_1, date_1, date_1,
					date_1, date_1, date_1, date_1, date_1, date_1,
				},
				"col_b": {
					date_2, date_2, date_2, date_2, date_2, date_2,
					date_2, date_2, date_2, date_2, date_2, date_2,
				},
			},
			expectedError: nil,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := require.New(t)
			var g solver.Generator
			g.Generate(tt.table, seed)
			tt.table.SortTimestamps()
			r.Equal(tt.expected, tt.table.Timestamps)
		})
	}
}
