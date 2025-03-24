package solver_test

import (
	"testing"

	"github.com/stretchr/testify/require"

	"github.com/phdah/sql-tdg/internals/solver"
	"github.com/phdah/sql-tdg/internals/table"
	"github.com/phdah/sql-tdg/internals/types"
)

func TestGenerator_Generate(t *testing.T) {
	tests := []struct {
		name          string
		col           string
		table         *table.Table
		expected      any
		expectedError error
	}{
		{
			name: "test this thing",
			col:  "col_a",
			table: table.NewTable([]types.Column{
				{
					Name: "col_a",
					Type: types.IntType,
					Constraints: []types.Constraints{
						solver.IntEq{10}, // Column should be all equal to 10
					},
				},
			}, 10),
			expected:      []int([]int{10, 10, 10, 10, 10, 10, 10, 10, 10, 10}),
			expectedError: nil,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := require.New(t)
			var g solver.Generator
			g.Generate(tt.table)
			result, err := tt.table.GetInts(tt.col)
			if err != nil {
				panic(err)
			}
			r.Equal(tt.expected, result)

		})
	}
}
