package parser

import (
	"github.com/alecthomas/participle/v2"
	"github.com/alecthomas/participle/v2/lexer"
)

/* ---------- Lexer ---------- */

var SqlLex = lexer.MustSimple([]lexer.SimpleRule{
	{Name: "WS", Pattern: `[ \t\r\n]+`},
	{Name: "LineComment", Pattern: `--[^\n]*`},

	{Name: "String", Pattern: `'([^']|'')*'|"([^"]|"")*"`},
	{Name: "Int", Pattern: `\d+`},
	{Name: "Ident", Pattern: `[A-Za-z_][A-Za-z0-9_]*`},

	// ONLY here for comparisons (order matters: this must come before Sym)
	{Name: "CmpOp", Pattern: `<=|>=|<>|!=|=|<|>`},

	// Punctuation (NO = < > here)
	{Name: "Sym", Pattern: `\*|,|\.|\(|\)`},
})

/* ---------- Grammar ---------- */

type Query struct {
	Select  *SelectClause `parser:"'SELECT' @@"`
	From    *FromClause   `parser:"'FROM' @@"`
	Joins   []*JoinClause `parser:"@@*"`
	Where   *Expr         `parser:"( 'WHERE' @@ )?"`
	Qualify *Expr         `parser:"( 'QUALIFY' @@ )?"`
}

type SelectClause struct {
	Items []*Expr `parser:"@@ ( ',' @@ )*"`
}
type FromClause struct {
	Table *QIdent `parser:"@@"`
}

type JoinClause struct {
	Type  *JoinType `parser:"@@? 'JOIN'"`
	Table *QIdent   `parser:"@@"`
	On    *Expr     `parser:"( 'ON' @@ )?"`
	Using []string  `parser:"( 'USING' '(' @Ident ( ',' @Ident )* ')' )?"`
}
type JoinType struct {
	Left  bool `parser:"  ( @'LEFT'  ( 'OUTER' )? )"`
	Right bool `parser:"| ( @'RIGHT' ( 'OUTER' )? )"`
	Full  bool `parser:"| ( @'FULL'  ( 'OUTER' )? )"`
	Inner bool `parser:"| @'INNER'"`
	Cross bool `parser:"| @'CROSS'"`
	Nat   bool `parser:"| @'NATURAL'"`
}

type QIdent struct {
	Parts []string `parser:"@Ident ( '.' @Ident )*"`
}

/* ---- Expressions (no left recursion) ---- */

type Expr struct {
	Left *And `parser:"@@"`
	Rest []*struct {
		Op    string `parser:"@'OR'"`
		Right *And   `parser:"@@"`
	} `parser:"@@*"`
}
type And struct {
	Left *Cmp `parser:"@@"`
	Rest []*struct {
		Op    string `parser:"@'AND'"`
		Right *Cmp   `parser:"@@"`
	} `parser:"@@*"`
}
type Cmp struct {
	Left  *Primary `parser:"@@"`
	Op    *string  `parser:"( @CmpOp"`
	Right *Primary `parser:"  @@ )?"`
}
type Primary struct {
	QIdent *QIdent `parser:"  @@"`
	Num    *string `parser:"| @Int"`
	Str    *string `parser:"| @String"`
	Paren  *Expr   `parser:"| '(' @@ ')'"`
	Func   *Func   `parser:"| @@"`
}
type Func struct {
	Name *QIdent `parser:"@@"`
	Args []*Expr `parser:"'(' ( @@ ( ',' @@ )* )? ')'"`
}

/* ---------- Build ---------- */

var Parser = participle.MustBuild[Query](
	participle.Lexer(SqlLex),
	participle.Elide("WS", "LineComment"),
	participle.CaseInsensitive(
		"select", "from", "where", "qualify", "join", "left", "right", "full", "outer", "inner", "cross", "natural", "on", "using", "and", "or",
	),
)
