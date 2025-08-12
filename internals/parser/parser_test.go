package parser_test

import (
	"testing"

	"github.com/phdah/sql-tdg/internals/parser"
	"github.com/stretchr/testify/require"
)

func TestParse_QuaryParsing(t *testing.T) {
	query := `
		SELECT x,y
		FROM t
		LEFT JOIN u ON t.x = u.p
		JOIN u ON t.x = u.p AND t.x = u.r
		CROSS JOIN t ON t.a > t.l
		NATURAL JOIN t ON t.a > t.l
		WHERE x > 5 OR y = 10 AND t AND x = 10 AND t = "2025-06-19"
	`
	// QUALIFY row_number() = 1
	q, err := parser.Parser.ParseString("", query)
	if err != nil {
		t.Fatalf("Failed parsing query:\n%s, err:\n%e", query, err)
	}
	wantJoins := []parser.JoinIR{
		{
			Kind:      "LEFT",
			Table:     "u",
			Condition: []parser.ConditionsIR{{Left: "t.x", Op: "=", Right: "u.p"}},
		},
		{
			Kind:  "INNER",
			Table: "u",
			Condition: []parser.ConditionsIR{
				{Left: "t.x", Op: "=", Right: "u.p"},
				{Left: "t.x", Op: "=", Right: "u.r"},
			},
		},
		{
			Kind:      "CROSS",
			Table:     "t",
			Condition: []parser.ConditionsIR{{Left: "t.a", Op: ">", Right: "t.l"}},
		},
		{
			Kind:      "NATURAL INNER",
			Table:     "t",
			Condition: []parser.ConditionsIR{{Left: "t.a", Op: ">", Right: "t.l"}},
		},
	}
	wantConditions := []parser.ConditionsIR{
		{
			Left:  "x",
			Op:    ">",
			Right: "5",
		},
		{
			Left:  "y",
			Op:    "=",
			Right: "10",
		},
		{
			Left:  "t",
			Op:    "bool",
			Right: "true",
		},
		{
			Left:  "x",
			Op:    "=",
			Right: "10",
		},
		{
			Left:  "t",
			Op:    "=",
			Right: `"2025-06-19"`,
		},
	}
	gotJoins := q.GetJoins()
	gotConditions := q.GetConditions()

	r := require.New(t)
	r.Equal(gotJoins, wantJoins)
	r.Equal(gotConditions, wantConditions)
}
