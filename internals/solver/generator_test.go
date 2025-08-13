package solver_test

import (
	"testing"
	"time"

	"github.com/apache/arrow/go/v14/arrow"
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
		expected      map[string][]int32
		expectedError error
	}{
		{
			name: "test with one column single condition",
			table: table.NewTable([]types.Column{
				{
					Name: "col_a",
					Type: types.IntType,
					Constraints: []types.Constraints{
						solver.IntEq{Value: 10},
					},
				},
			}, 12),
			expected: map[string][]int32{
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
						solver.IntNEq{Value: 10},
						solver.IntGt{Value: 3},
						solver.IntLt{Value: 100},
					},
				},
			}, 12),
			expected: map[string][]int32{
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
						solver.IntEq{Value: 10},
					},
				},
				{
					Name: "col_b",
					Type: types.IntType,
					Constraints: []types.Constraints{
						solver.IntEq{Value: 3},
					},
				},
			}, 12),
			expected: map[string][]int32{
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

			// Call BuildInts to finalize the Arrow arrays from the builders
			tt.table.BuildInts()
			tt.table.SortInts()

			// Create a map to hold the actual Go slices from the Arrow arrays
			actual := make(map[string][]int32)
			for colName := range tt.expected {
				arr, _ := tt.table.GetInts(colName)
				if arr != nil {
					actual[colName] = arr.Int32Values()
				}
			}
			r.Equal(tt.expected, actual)
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
		expected      map[string][]time.Time
		expectedError error
	}{
		{
			name: "test with one column single condition",
			table: table.NewTable([]types.Column{
				{
					Name: "col_a",
					Type: types.TimestampType,
					Constraints: []types.Constraints{
						solver.IntEq{Value: solver.ToDate("2013-06-17")},
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
						solver.IntNEq{Value: solver.ToTimestamp("2013-06-17T15:21:00Z")},
						solver.IntGt{Value: solver.ToTimestamp("2013-06-17T15:10:00Z")},
						solver.IntLt{Value: solver.ToTimestamp("2013-06-17T15:45:00Z")},
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
						solver.IntEq{Value: solver.ToDate("2013-06-17")},
					},
				},
				{
					Name: "col_b",
					Type: types.TimestampType,
					Constraints: []types.Constraints{
						solver.IntEq{Value: solver.ToTimestamp("2013-06-17T15:21:00Z")},
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
			tt.table.BuildTimestamps()
			tt.table.SortTimestamps()
			actual := make(map[string][]time.Time)
			for colName := range tt.expected {
				arr, _ := tt.table.GetTimestamps(colName)
				if arr != nil {
					actual[colName] = make([]time.Time, arr.Len())
					for i := 0; i < arr.Len(); i++ {
						actual[colName][i] = arr.Value(i).ToTime(arrow.Microsecond)
					}
				}
			}
			r.Equal(tt.expected, actual)
		})
	}
}

func TestBoolGenerator_Generate(t *testing.T) {
	seed := int64(42)
	tests := []struct {
		name          string
		table         *table.Table
		expected      any
		expectedError error
	}{
		{
			name: "test with one column true",
			table: table.NewTable([]types.Column{
				{
					Name: "col_a",
					Type: types.BoolType,
					Constraints: []types.Constraints{
						solver.BoolTrue{},
					},
				},
			}, 12),
			expected: map[string][]bool{
				"col_a": {
					true, true, true, true, true, true,
					true, true, true, true, true, true,
				},
			},
			expectedError: nil,
		},
		{
			name: "test with one column false",
			table: table.NewTable([]types.Column{
				{
					Name: "col_a",
					Type: types.BoolType,
					Constraints: []types.Constraints{
						solver.BoolFalse{},
					},
				},
			}, 12),
			expected: map[string][]bool{
				"col_a": {
					false, false, false, false, false, false,
					false, false, false, false, false, false,
				},
			},
			expectedError: nil,
		},
		{
			name: "test with two columns",
			table: table.NewTable([]types.Column{
				{
					Name: "col_a",
					Type: types.BoolType,
					Constraints: []types.Constraints{
						solver.BoolTrue{},
					},
				},
				{
					Name: "col_b",
					Type: types.BoolType,
					Constraints: []types.Constraints{
						solver.BoolFalse{},
					},
				},
			}, 12),
			expected: map[string][]bool{
				"col_a": {
					true, true, true, true, true, true,
					true, true, true, true, true, true,
				},
				"col_b": {
					false, false, false, false, false, false,
					false, false, false, false, false, false,
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
			r.Equal(tt.expected, tt.table.Bools)
		})
	}
}
