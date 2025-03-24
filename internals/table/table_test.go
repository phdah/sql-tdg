package table_test

import (
	"testing"

	"github.com/phdah/sql-tdg/internals/solver"
	"github.com/phdah/sql-tdg/internals/table"
	"github.com/phdah/sql-tdg/internals/types"
	"github.com/stretchr/testify/require"
)

func TestTable_Append(t *testing.T) {
	tests := []struct {
		name    string // description of this test case
		columns []types.Column
		rows    int
		col     string
		val     int
	}{
		{
			name: "test append",
			columns: []types.Column{
				{
					Name: "col_a",
					Type: types.IntType,
					Constraints: []types.Constraints{
						solver.IntEq{},
					},
				},
			},
			rows: 1,
			col:  "col_a",
			val:  10,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := require.New(t)
			ta := table.NewTable(tt.columns, tt.rows)
			err := ta.Append(tt.col, tt.val)
			r.NoError(err)
		})
	}
}

func TestTable_Wipe(t *testing.T) {
	tests := []struct {
		name     string // description of this test case
		columns  []types.Column
		rows     int
		col      string
		expected []int
	}{
		{
			name: "test append",
			columns: []types.Column{
				{
					Name: "col_a",
					Type: types.IntType,
					Constraints: []types.Constraints{
						solver.IntEq{},
					},
				},
			},
			rows:     1,
			col:      "col_a",
			expected: []int([]int(nil)),
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := require.New(t)
			ta := table.NewTable(tt.columns, tt.rows)
			ta.Wipe()
			result, _ := ta.GetInts(tt.col)
			r.Equal(tt.expected, result)
		})
	}
}
