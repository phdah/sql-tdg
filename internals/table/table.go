package table

import (
	"sort"
	"sync"
	"time"

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

	Ints       map[string][]int
	Timestamps map[string][]time.Time

	muInts       sync.Mutex
	muTimestamps sync.Mutex
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
		Schema:     schema,
		Types:      getColTypes(schema),
		Dim:        Dim{Rows: rows, Cols: len(schema)},
		Ints:       make(map[string][]int),
		Timestamps: make(map[string][]time.Time),
	}
}

func (t *Table) Append(col string, val any) error {
	switch t.Types[col] {
	case types.IntType:
		t.muInts.Lock()
		t.Ints[col] = append(t.Ints[col], val.(int))
		t.muInts.Unlock()
	case types.TimestampType:
		t.muTimestamps.Lock()
		t.Timestamps[col] = append(t.Timestamps[col], val.(time.Time))
		t.muTimestamps.Unlock()
	}
	return nil
}

func (t *Table) GetInts(col string) ([]int, error) {
	t.muInts.Lock()
	defer t.muInts.Unlock()
	return t.Ints[col], nil
}

func (t *Table) SortInts() {
	t.muInts.Lock()
	defer t.muInts.Unlock()
	for _, col := range t.Schema {
		if col.Type == types.IntType {
			sort.Ints(t.Ints[col.Name])
		}
	}
}

func (t *Table) SortTimestamps() {
	t.muTimestamps.Lock()
	defer t.muTimestamps.Unlock()
	for _, col := range t.Schema {
		if col.Type == types.TimestampType {
			slice := t.Timestamps[col.Name]
			sort.Slice(slice, func(i int, j int) bool {
				return slice[i].Before(slice[j])
			})
		}
	}
}

func (t *Table) Wipe() error {
	t.muInts.Lock()
	defer t.muInts.Unlock()
	t.Ints = make(map[string][]int)
	return nil
}
