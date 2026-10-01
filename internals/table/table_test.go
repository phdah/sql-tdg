package table_test

import (
	"testing"
	"time"

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
		val     any
	}{
		{
			name: "append integer",
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
			val:  int32(10),
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := require.New(t)
			ta := table.NewTable(tt.columns, tt.rows)
			r.NoError(ta.Append(tt.col, tt.val))
		})
	}
}

func TestTable_AppendRejectsWrongType(t *testing.T) {
	ta := table.NewTable([]types.Column{{Name: "col_a", Type: types.IntType}}, 1)

	err := ta.Append("col_a", 10)

	require.Error(t, err)
}

func TestTable_Wipe(t *testing.T) {
	ta := table.NewTable([]types.Column{
		{Name: "int_col", Type: types.IntType},
		{Name: "timestamp_col", Type: types.TimestampType},
		{Name: "bool_col", Type: types.BoolType},
		{Name: "string_col", Type: types.StringType},
	}, 1)

	require.NoError(t, ta.Append("int_col", int32(10)))
	require.NoError(t, ta.Append("timestamp_col", time.Unix(10, 0).UTC()))
	require.NoError(t, ta.Append("bool_col", true))
	require.NoError(t, ta.Append("string_col", "value"))
	ta.BuildInts()
	ta.BuildTimestamps()
	ta.BuildBools()
	ta.BuildStrings()

	require.NoError(t, ta.Wipe())

	ints, err := ta.GetAllInts()
	require.NoError(t, err)
	require.Equal(t, map[string][]int32{"int_col": nil}, ints)
	timestamps, err := ta.GetAllTimestamps()
	require.NoError(t, err)
	require.Equal(t, map[string][]time.Time{"timestamp_col": nil}, timestamps)
	bools, err := ta.GetAllBools()
	require.NoError(t, err)
	require.Equal(t, map[string][]bool{"bool_col": nil}, bools)
	strings, err := ta.GetAllStrings()
	require.NoError(t, err)
	require.Equal(t, map[string][]string{"string_col": nil}, strings)
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
					r.NoError(ta.Append(colName, val))
				}
			}

			ta.BuildInts()
			ta.SortInts()
			got, err := ta.GetAllInts()
			r.NoError(err)
			r.Equal(tt.expected, got)
		})
	}
}

func TestTable_AllColumnTypesUseArrowStorage(t *testing.T) {
	ts := time.Date(2026, time.October, 1, 9, 30, 0, 0, time.UTC)
	ta := table.NewTable([]types.Column{
		{Name: "int_col", Type: types.IntType},
		{Name: "timestamp_col", Type: types.TimestampType},
		{Name: "bool_col", Type: types.BoolType},
		{Name: "string_col", Type: types.StringType},
	}, 2)

	require.NoError(t, ta.Append("int_col", int32(7)))
	require.NoError(t, ta.Append("int_col", int32(3)))
	require.NoError(t, ta.Append("timestamp_col", ts))
	require.NoError(t, ta.Append("timestamp_col", ts.Add(time.Second)))
	require.NoError(t, ta.Append("bool_col", true))
	require.NoError(t, ta.Append("bool_col", false))
	require.NoError(t, ta.Append("string_col", "alpha"))
	require.NoError(t, ta.Append("string_col", "beta"))

	ta.BuildInts()
	ta.BuildTimestamps()
	ta.BuildBools()
	ta.BuildStrings()

	ints, err := ta.GetAllInts()
	require.NoError(t, err)
	require.Equal(t, map[string][]int32{"int_col": {7, 3}}, ints)

	timestamps, err := ta.GetAllTimestamps()
	require.NoError(t, err)
	require.Equal(t, map[string][]time.Time{"timestamp_col": {ts, ts.Add(time.Second)}}, timestamps)

	bools, err := ta.GetAllBools()
	require.NoError(t, err)
	require.Equal(t, map[string][]bool{"bool_col": {true, false}}, bools)

	strings, err := ta.GetAllStrings()
	require.NoError(t, err)
	require.Equal(t, map[string][]string{"string_col": {"alpha", "beta"}}, strings)
}
