package parser_test

import (
	"fmt"
	"testing"

	"github.com/phdah/sql-tdg/internals/parser"
)

func TestParse_WhereClauseVisitor(t *testing.T) {
	query := `
		SELECT x,y
		FROM t
		LEFT JOIN u ON t.x = u.p
		JOIN u ON t.x = u.p
		CROSS JOIN t ON t.a > t.l
		NATURAL JOIN t ON t.a > t.l
		WHERE x > 5
	`
	// QUALIFY row_number() = 1
	q, err := parser.Parser.ParseString("", query)
	if err != nil {
		t.Fatalf("Failed parsing query:\n%s, err:\n%e", query, err)
	}
	joins := q.GetJoins()
	fmt.Println(joins)
}
