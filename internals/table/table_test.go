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

func TestTable_SortInts(t *testing.T) {
	tests := []struct {
		name     string // description of this test case
		table    *table.Table
		expected map[string][]int
	}{
		{
			name: "single list of ints",
			table: &table.Table{
				Schema: []types.Column{
					{
						Name: "col1",
						Type: types.IntType,
					},
				},
				Ints: map[string][]int{
					"col1": {7, 2, 6, 3, 1},
				},
			},
			expected: map[string][]int{
				"col1": {1, 2, 3, 6, 7},
			},
		},
		{
			name: "multiple int columns",
			table: &table.Table{
				Schema: []types.Column{
					{
						Name: "col1",
						Type: types.IntType,
					},
					{
						Name: "col2",
						Type: types.IntType,
					},
				},
				Ints: map[string][]int{
					"col1": {5, 3, 9},
					"col2": {2, 2, 1},
				},
			},
			expected: map[string][]int{
				"col1": {3, 5, 9},
				"col2": {1, 2, 2},
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := require.New(t)
			tt.table.SortInts()
			for col, result := range tt.table.Ints {
				r.Equal(tt.expected[col], result)
			}
		})
	}
}
