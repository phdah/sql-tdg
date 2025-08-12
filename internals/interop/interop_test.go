package interop_test

import (
	"testing"

	"github.com/stretchr/testify/require"

	"github.com/phdah/sql-tdg/internals/interop"
	"github.com/phdah/sql-tdg/internals/parser"
	"github.com/phdah/sql-tdg/internals/solver"
	"github.com/phdah/sql-tdg/internals/table"
	"github.com/phdah/sql-tdg/internals/types"
)

func TestInterop_FullQueryGenerator(t *testing.T) {
	wantJoins := []parser.JoinIR{}
	gotJoins := []parser.JoinIR{}
	r := require.New(t)
	r.Equal(gotJoins, wantJoins)

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
			query: "SELECT col_a FROM t WHERE col_a = 10",
			table: table.NewTable([]types.Column{
				{
					Name:        "col_a",
					Type:        types.IntType,
					Constraints: nil,
				},
			}, 12),
			expected: map[string][]int{
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
			expected: map[string][]int{
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
			tt.table.SortInts()
			r.Equal(tt.expected, tt.table.Ints)
		})
	}
}
