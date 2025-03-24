package table

import (
	"sync"

	"github.com/phdah/sql-tdg/internals/types"
)

type Dim struct {
	Rows int
	Cols int
}

type Table struct {
	Schema []types.Column
	Types  map[string]types.Type
	Dim    Dim

	Ints map[string][]int

	muInts sync.Mutex
}

func getColTypes(schema []types.Column) map[string]types.Type {
	types := make(map[string]types.Type)
	for _, col := range schema {
		types[col.Name] = col.Type
	}
	return types
}

func NewTable(schema []types.Column, rows int) *Table {
	return &Table{
		Schema: schema,
		Types:  getColTypes(schema),
		Dim:    Dim{Rows: rows, Cols: len(schema)},
		Ints:   make(map[string][]int),
	}
}

func (t *Table) Append(col string, val any) error {
	switch t.Types[col] {
	case types.IntType:
		t.muInts.Lock()
		t.Ints[col] = append(t.Ints[col], val.(int))
		t.muInts.Unlock()
	}
	return nil
}

func (t *Table) GetInts(col string) ([]int, error) {
	t.muInts.Lock()
	defer t.muInts.Unlock()
	return t.Ints[col], nil
}

func (t *Table) Wipe() error {
	t.muInts.Lock()
	defer t.muInts.Unlock()
	t.Ints = make(map[string][]int)
	return nil
}
