package interop_test

import (
	"testing"
	"time"

	"github.com/stretchr/testify/require"

	"github.com/phdah/sql-tdg/internals/interop"
	"github.com/phdah/sql-tdg/internals/parser"
	"github.com/phdah/sql-tdg/internals/solver"
	"github.com/phdah/sql-tdg/internals/table"
	"github.com/phdah/sql-tdg/internals/types"
)

func TestInterop_FullQueryGeneratorInts(t *testing.T) {
	seed := int64(42)
	tests := []struct {
		name          string
		query         string
		table         *table.Table
		expected      map[string][]int32
		expectedError error
	}{
		{
			name:  "test with one column single condition",
			query: "SELECT col_a FROM t WHERE col_a = 10",
			table: table.NewTable([]types.Column{
				{
					Name:        "col_a",
					Type:        types.IntType,
					Constraints: nil,
				},
			}, 12),
			expected: map[string][]int32{
				"col_a": {10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10},
			},
			expectedError: nil,
		},
		{
			name:  "test with two column multi conditions",
			query: "SELECT col_a, col_b FROM t WHERE col_a > 5 OR col_a = 10 AND col_b = 5",
			table: table.NewTable([]types.Column{
				{
					Name:        "col_a",
					Type:        types.IntType,
					Constraints: nil,
				},
				{
					Name:        "col_b",
					Type:        types.IntType,
					Constraints: nil,
				},
			}, 12),
			expected: map[string][]int32{
				"col_a": {10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10},
				"col_b": {5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5},
			},
			expectedError: nil,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := require.New(t)

			q, err := parser.Parser.ParseString("", tt.query)
			if err != nil {
				t.Fatalf("Failed parsing query:\n%s, err:\n%e", tt.query, err)
			}

			interopQuery := interop.Wrap(q)
			var g solver.Generator
			err = interopQuery.AddConditions(tt.table)
			if err != nil {
				t.Fatalf("Failed parsing query:\n%s, err:\n%e", tt.query, err)
			}
			g.Generate(tt.table, seed)

			// The table's data is still in builders, so we need to build the final Arrow arrays first.
			tt.table.BuildInts()

			// Sort is now optional depending on the test, but let's keep it to ensure it works.
			tt.table.SortInts()
			got, err := tt.table.GetAllInts()
			if err != nil {
				t.Fatalf("Failed getting integer columns, err:\n%e", err)
			}
			r.Equal(tt.expected, got)
		})
	}
}

func TestInterop_FullQueryGeneratorBool(t *testing.T) {
	seed := int64(42)
	tests := []struct {
		name          string
		query         string
		table         *table.Table
		expected      any
		expectedError error
	}{
		{
			name:  "test with one column single condition",
			query: "SELECT col_a FROM t WHERE col_a",
			table: table.NewTable([]types.Column{
				{
					Name:        "col_a",
					Type:        types.BoolType,
					Constraints: nil,
				},
			}, 12),
			expected: map[string][]bool{
				"col_a": {true, true, true, true, true, true, true, true, true, true, true, true},
			},
			expectedError: nil,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := require.New(t)

			q, err := parser.Parser.ParseString("", tt.query)
			if err != nil {
				t.Fatalf("Failed parsing query:\n%s, err:\n%e", tt.query, err)
			}

			interopQuery := interop.Wrap(q)
			var g solver.Generator
			err = interopQuery.AddConditions(tt.table)
			if err != nil {
				t.Fatalf("Failed parsing query:\n%s, err:\n%e", tt.query, err)
			}
			g.Generate(tt.table, seed)
			r.Equal(tt.expected, tt.table.Bools)
		})
	}
}

func TestInterop_FullQueryGeneratorTimestamp(t *testing.T) {
	seed := int64(42)
	date_1 := solver.FromInt(solver.ToDate("2013-06-17"))
	date_2 := solver.FromInt(solver.ToTimestamp("2013-06-17T14:29:00Z"))
	tests := []struct {
		name          string
		query         string
		table         *table.Table
		expected      map[string][]time.Time
		expectedError error
	}{
		{
			name:  "test with one column single condition",
			query: `SELECT col_a FROM t WHERE col_a = '2013-06-17'`,
			table: table.NewTable([]types.Column{
				{
					Name:        "col_a",
					Type:        types.TimestampType,
					Constraints: nil,
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
			name:  "test with one column single condition",
			query: `SELECT col_a FROM t WHERE col_a = "2013-06-17T14:29:00Z"`,
			table: table.NewTable([]types.Column{
				{
					Name:        "col_a",
					Type:        types.TimestampType,
					Constraints: nil,
				},
			}, 12),
			expected: map[string][]time.Time{
				"col_a": {
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

			q, err := parser.Parser.ParseString("", tt.query)
			if err != nil {
				t.Fatalf("Failed parsing query:\n%s, err:\n%e", tt.query, err)
			}

			interopQuery := interop.Wrap(q)
			var g solver.Generator
			err = interopQuery.AddConditions(tt.table)
			if err != nil {
				t.Fatalf("Failed parsing query:\n%s, err:\n%e", tt.query, err)
			}
			g.Generate(tt.table, seed)

			// Finalize the Arrow timestamp arrays
			tt.table.BuildTimestamps()
			got, err := tt.table.GetAllTimestamps()
			if err != nil {
				t.Fatalf("Failed getting timestamp columns, err:\n%e", err)
			}
			r.Equal(tt.expected, got)
		})
	}
}
