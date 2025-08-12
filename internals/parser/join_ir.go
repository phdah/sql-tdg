package parser

// JoinKind represents the type of SQL JOIN operation. The values correspond
// to the different JOIN syntax variants that can appear in a SELECT
// statement.
//
// The string values are the canonical names used in the generated
// intermediate representation (IR).
type JoinKind string

const (
	// Inner represents a plain INNER JOIN or an implicit join with no
	// explicit keyword.
	Inner JoinKind = "INNER"

	// Left represents a LEFT JOIN.
	Left JoinKind = "LEFT"

	// Right represents a RIGHT JOIN.
	Right JoinKind = "RIGHT"

	// Full represents a FULL JOIN.
	Full JoinKind = "FULL"

	// Cross represents a CROSS JOIN.
	Cross JoinKind = "CROSS"

	// NatIn represents a NATURAL INNER JOIN.
	NatIn JoinKind = "NATURAL INNER"

	// NatL represents a NATURAL LEFT JOIN.
	NatL JoinKind = "NATURAL LEFT"

	// NatR represents a NATURAL RIGHT JOIN.
	NatR JoinKind = "NATURAL RIGHT"

	// NatF represents a NATURAL FULL JOIN.
	NatF JoinKind = "NATURAL FULL"
)

// JoinIR is the intermediate representation of a JOIN clause.
// It contains the join kind, the name of the joined table, and the
// conditions used to match rows.
type JoinIR struct {
	Kind      string
	Table     string
	Condition []ConditionsIR
}

// GetKind returns the JoinKind corresponding to the flags set on the
// JoinType value. It interprets the presence of Cross, Nat, Left, Right,
// Full, and Inner flags in the following order:
//
//   1. If the JoinType is nil, Inner is returned (default join).
//   2. If Cross is true, a CROSS JOIN is returned.
//   3. If Nat is true, one of the NATURAL JOIN variants is returned
//      based on which side flag is set (Left, Right, Full, or Inner).
//   4. If none of the above, the first matching side flag is used
//      (Left, Right, Full, or Inner). If no flags are set, Inner is
//      returned.
func (jt *JoinType) GetKind() JoinKind {
	if jt == nil {
		return Inner
	} // plain JOIN => INNER
	if jt.Cross {
		return Cross
	}
	if jt.Nat {
		switch {
		case jt.Left:
			return NatL
		case jt.Right:
			return NatR
		case jt.Full:
			return NatF
		case jt.Inner:
			return NatIn
		default:
			return NatIn // NATURAL JOIN => NATURAL INNER
		}
	}
	switch {
	case jt.Left:
		return Left
	case jt.Right:
		return Right
	case jt.Full:
		return Full
	case jt.Inner:
		return Inner
	default:
		return Inner // no prefix => INNER
	}
}

// GetJoin converts a JoinClause into its intermediate representation
// (JoinIR). It extracts the kind, the table name, and the ON
// condition expressed as a ConditionsIR.
func (j *JoinClause) GetJoin() JoinIR {
	return JoinIR{
		Kind:      string(j.Type.GetKind()),
		Table:     j.Table.Parts[0],
		Condition: j.On.ToIR(),
	}
}

// GetJoins returns a slice of JoinIR objects representing all JOIN
// clauses in the Query. It iterates over the Query's Joins field,
// converting each to JoinIR via JoinClause.GetJoin.
func (q *Query) GetJoins() []JoinIR {
	out := make([]JoinIR, 0, len(q.Joins))
	for _, j := range q.Joins {
		out = append(out, j.GetJoin())
	}
	return out
}
