package parser

import "strings"

// LeftIR represents the left side of a condition in the IR.
type LeftIR string

// OpIR represents the operator of a condition in the IR.
type OpIR string

// RightIR represents the right side of a condition in the IR.
type RightIR string

// ConditionsIR is a lightweight representation of a single condition,
// containing the left operand, the operator, and the right operand.
type ConditionsIR struct {
	Left  LeftIR
	Op    OpIR
	Right RightIR
}

// primaryAtom converts a Primary expression into its string representation.
// It handles qualified identifiers, numeric literals, string literals,
// and function calls. If the Primary is nil or does not contain a
// recognizable value, it returns an empty string.
func primaryAtom(p *Primary) string {
	if p == nil {
		return ""
	}
	if p.QIdent != nil && len(p.QIdent.Parts) > 0 {
		return strings.Join(p.QIdent.Parts, ".")
	}
	if p.Num != nil {
		return *p.Num
	}
	if p.Str != nil {
		return *p.Str
	}
	if p.Func != nil {
		return strings.Join(p.Func.Name.Parts, ".") + "()"
	}
	return ""
}

// ToIR converts an Expr into a slice of ConditionsIR, representing the
// intermediate form of the expression. It walks the expression tree,
// extracting each condition and preserving logical operators.
func (e *Expr) ToIR() []ConditionsIR {
	conditions := make([]ConditionsIR, 0)
	conditions = append(conditions, e.Left.ToIR()...)

	for _, orTerm := range e.Rest {
		conditions = append(conditions, orTerm.Right.ToIR()...)
	}
	return conditions
}

// ToIR converts an And expression into a slice of ConditionsIR,
// extracting each conjunctive condition. It handles the leftmost term
// separately and then iterates over any additional terms.
func (a *And) ToIR() []ConditionsIR {
	conditions := make([]ConditionsIR, 0)

	if a.Left != nil {
		if a.Left.Op != nil {
			conditions = append(conditions, ConditionsIR{
				Left:  LeftIR(primaryAtom(a.Left.Left)),
				Op:    OpIR(*a.Left.Op),
				Right: RightIR(primaryAtom(a.Left.Right)),
			})
		} else {
			conditions = append(conditions, ConditionsIR{
				Left:  LeftIR(primaryAtom(a.Left.Left)),
				Op:    OpIR("bool"),
				Right: RightIR("true"),
			})
		}
	}

	for _, andTerm := range a.Rest {
		if andTerm.Right != nil {
			if andTerm.Right.Op != nil {
				conditions = append(conditions, ConditionsIR{
					Left:  LeftIR(primaryAtom(andTerm.Right.Left)),
					Op:    OpIR(*andTerm.Right.Op),
					Right: RightIR(primaryAtom(andTerm.Right.Right)),
				})
			} else {
				conditions = append(conditions, ConditionsIR{
					Left:  LeftIR(primaryAtom(andTerm.Right.Left)),
					Op:    OpIR("bool"),
					Right: RightIR("true"),
				})
			}
		}
	}
	return conditions
}

// GetConditions extracts all condition clauses from a Query, including
// both the WHERE and QUALIFY clauses. It returns a flat slice of
// ConditionsIR representing every condition in the query.
func (q *Query) GetConditions() []ConditionsIR {
	out := make([]ConditionsIR, 0)
	if q.Where != nil {
		out = append(out, q.Where.ToIR()...)
	}
	if q.Qualify != nil {
		out = append(out, q.Qualify.ToIR()...)
	}
	return out
}
