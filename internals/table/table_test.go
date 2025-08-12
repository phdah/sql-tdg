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
		name    string
		columns []types.Column
		rows    int
		col     string
		val     int32
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
		name     string
		columns  []types.Column
		rows     int
		col      string
		expected []int32
	}{
		{
			name: "test wipe",
			columns: []types.Column{
				{
					Name: "col_a",
					Type: types.IntType,
				},
			},
			rows:     1,
			col:      "col_a",
			expected: nil,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := require.New(t)
			ta := table.NewTable(tt.columns, tt.rows)
			ta.Wipe()
			resultArr, _ := ta.GetInts(tt.col)

			if resultArr == nil {
				r.Nil(tt.expected)
			} else {
				resultSlice := resultArr.Int32Values()
				r.Equal(tt.expected, resultSlice)
			}
		})
	}
}

func TestTable_SortInts(t *testing.T) {
	tests := []struct {
		name     string
		schema   []types.Column
		input    map[string][]int32
		expected map[string][]int32
	}{
		{
			name: "single list of ints",
			schema: []types.Column{
				{Name: "col1", Type: types.IntType},
			},
			input: map[string][]int32{
				"col1": {7, 2, 6, 3, 1},
			},
			expected: map[string][]int32{
				"col1": {1, 2, 3, 6, 7},
			},
		},
		{
			name: "multiple int columns",
			schema: []types.Column{
				{Name: "col1", Type: types.IntType},
				{Name: "col2", Type: types.IntType},
			},
			input: map[string][]int32{
				"col1": {5, 3, 9},
				"col2": {2, 2, 1},
			},
			expected: map[string][]int32{
				"col1": {3, 5, 9},
				"col2": {1, 2, 2},
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := require.New(t)

			ta := table.NewTable(tt.schema, len(tt.input[tt.schema[0].Name]))

			for colName, values := range tt.input {
				for _, val := range values {
					ta.Append(colName, val)
				}
			}

			ta.BuildInts()

			ta.SortInts()

			for col, expectedSlice := range tt.expected {
				resultArr, _ := ta.GetInts(col)
				r.NotNil(resultArr)
				r.Equal(expectedSlice, resultArr.Int32Values())
			}
		})
	}
}
