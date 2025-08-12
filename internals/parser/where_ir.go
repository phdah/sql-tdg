package parser

import "strings"

// Parser IR type aliases for readability.
// LeftIR represents the left-hand side of a condition.
// OpIR   represents the operator.
// RightIR represents the right-hand side of a condition.
type (
	LeftIR  string
	OpIR    string
	RightIR string
)

// ConditionsIR is a simple IR representation of a binary
// condition consisting of a left operand, an operator
// and a right operand.
type ConditionsIR struct {
	Left  LeftIR
	Op    OpIR
	Right RightIR
}

// primaryAtom turns a Primary AST node into a single
// atom string (e.g. identifier, numeric literal or
// string literal). It returns an empty string if the
// Primary is nil or has no recognizable value.
func primaryAtom(p *Primary) string {
	if p == nil {
		return ""
	}
	if len(p.QIdent) > 0 {
		return strings.Join(p.QIdent, ".")
	}
	if p.Num != nil {
		return *p.Num
	}
	if p.Str != nil {
		return *p.Str
	}
	return ""
}

// GetLeftIR extracts the left-hand side of the expression
// and returns it as a LeftIR. If the expression's left
// side is nil, an empty string is returned.
func (e *Expr) GetLeftIR() LeftIR {
	return LeftIR(primaryAtom(e.Left))
}

// GetOpIR extracts the operator from the expression.
// If the operator is nil, an empty OpIR is returned.
func (e *Expr) GetOpIR() OpIR {
	if e.Op == nil {
		return OpIR("")
	}
	return OpIR(*e.Op)
}

// GetRightIR extracts the right-hand side of the expression
// and returns it as a RightIR. If the expression's right
// side is nil, an empty string is returned.
func (e *Expr) GetRightIR() RightIR {
	return RightIR(primaryAtom(e.Right))
}

// ToIR converts an Expr into its ConditionsIR
// representation, aggregating the left, operator and
// right components.
func (e *Expr) ToIR() ConditionsIR {
	return ConditionsIR{
		Left:  e.GetLeftIR(),
		Op:    e.GetOpIR(),
		Right: e.GetRightIR(),
	}
}

// GetConditions collects simple (left, op, right) conditions
// from a Query's WHERE and QUALIFY clauses and returns them
// as a slice of ConditionsIR. The slice may contain zero,
// one or two elements depending on which clauses are present.
func (q *Query) GetConditions() []ConditionsIR {
	out := make([]ConditionsIR, 0, 2)
	// TODO: have it iterate over all where
	if q.Where != nil {
		out = append(out, q.Where.ToIR())
	}
	// TODO: have it iterate over all qualify
	if q.Qualify != nil {
		out = append(out, q.Qualify.ToIR())
	}
	return out
}
