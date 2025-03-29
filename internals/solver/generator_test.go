package solver_test

import (
	"testing"

	"github.com/stretchr/testify/require"

	"github.com/phdah/sql-tdg/internals/solver"
	"github.com/phdah/sql-tdg/internals/table"
	"github.com/phdah/sql-tdg/internals/types"
)

func TestGenerator_Generate(t *testing.T) {
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
			}, 10),
			expected: map[string][]int{
				"col_a": {10, 10, 10, 10, 10, 10, 10, 10, 10, 10},
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
			}, 10),
			expected: map[string][]int{
				"col_a": {28, 42, 48, 57, 58, 64, 73, 84, 90, 95},
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
			}, 10),
			expected: map[string][]int{
				"col_a": {10, 10, 10, 10, 10, 10, 10, 10, 10, 10},
				"col_b": {3, 3, 3, 3, 3, 3, 3, 3, 3, 3},
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
