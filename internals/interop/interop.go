package interop

import (
	"fmt"
	"strconv"

	"github.com/phdah/sql-tdg/internals/parser"
	"github.com/phdah/sql-tdg/internals/solver"
	"github.com/phdah/sql-tdg/internals/table"
	"github.com/phdah/sql-tdg/internals/types"
)

type Query struct {
	*parser.Query // embed to forward access
}

func Wrap(q *parser.Query) Query { return Query{q} }

func (q *Query) AddConditions(t *table.Table) error {
	// quick index by column name
	idx := make(map[string]int, len(t.Schema))
	for i := range t.Schema {
		idx[t.Schema[i].Name] = i
	}

	constraints := q.GetConditions()
	colTypes := t.Types
	for _, c := range constraints {
		i, ok := idx[string(c.Left)]
		if !ok {
			return fmt.Errorf("unknown column %q", c.Left)
		}
		col := &t.Schema[i]
		cons, err := MakeConstraint(colTypes[string(c.Left)], c)
		if err != nil {
			return fmt.Errorf("column %s: %w", c.Left, err)
		}
		col.Constraints = append(col.Constraints, cons)
	}
	return nil
}

func MakeConstraint(typ types.Type, c parser.ConditionsIR) (types.Constraints, error) {
	switch typ {
	case types.IntType:
		n, err := strconv.Atoi(string(c.Right))
		if err != nil {
			return nil, fmt.Errorf("int parse: %w", err)
		}
		switch c.Op {
		case "=":
			return solver.IntEq{Value: n}, nil
		case "!=":
			return solver.IntNEq{Value: n}, nil
		case ">":
			return solver.IntGt{Value: n}, nil
		case ">=":
			return solver.IntGte{Value: n}, nil
		case "<":
			return solver.IntLt{Value: n}, nil
		case "<=":
			return solver.IntLte{Value: n}, nil
		default:
			return nil, fmt.Errorf("bad int op %q", c.Op)
		}

	// case types.BoolType:
	// 	b, err := strconv.ParseBool(val)
	// 	if err != nil {
	// 		return nil, fmt.Errorf("bool parse: %w", err)
	// 	}
	// 	switch op {
	// 	case "=":
	// 		return solver.BoolTrue{Value: b}, nil
	// 	case "!=":
	// 		return solver.BoolFalse{Value: b}, nil
	// 	default:
	// 		return nil, fmt.Errorf("bad bool op %q", op)
	// 	}

	// case types.TimestampType:
	// 	// pick your accepted layout(s)
	// 	ts, err := time.Parse(time.RFC3339, val)
	// 	if err != nil {
	// 		return nil, fmt.Errorf("time parse (RFC3339): %w", err)
	// 	}
	// 	switch op {
	// 	case "=":
	// 		return solver.TimeEq{Value: ts}, nil
	// 	case "!=":
	// 		return solver.TimeNe{Value: ts}, nil
	// 	case ">":
	// 		return solver.TimeGt{Value: ts}, nil
	// 	case ">=":
	// 		return solver.TimeGe{Value: ts}, nil
	// 	case "<":
	// 		return solver.TimeLt{Value: ts}, nil
	// 	case "<=":
	// 		return solver.TimeLe{Value: ts}, nil
	// 	default:
	// 		return nil, fmt.Errorf("bad time op %q", op)
	// 	}
	default:
		return nil, fmt.Errorf("unsupported column type %v", typ)
	}
}
