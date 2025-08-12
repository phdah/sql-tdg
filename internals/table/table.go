package table

import (
	"sort"
	"sync"
	"time"

	"github.com/phdah/sql-tdg/internals/types"

	// "github.com/apache/arrow/go/v14/arrow"
	"github.com/apache/arrow/go/v14/arrow/array"
	"github.com/apache/arrow/go/v14/arrow/memory"
	"slices"
)

type Dim struct {
	Rows int
	Cols int
}

type Table struct {
	Schema []types.Column
	Types  map[string]types.Type
	Dim    Dim

	Ints       map[string]*array.Int32 // Updated to use Arrow array
	Timestamps map[string][]time.Time
	Bools      map[string][]bool

	// We'll need a map of builders to handle appending new integer data
	IntBuilders map[string]*array.Int32Builder
	mem         memory.Allocator

	muInts       sync.Mutex
	muTimestamps sync.Mutex
	muBools      sync.Mutex
}

func getColTypes(schema []types.Column) map[string]types.Type {
	types := make(map[string]types.Type)
	for _, col := range schema {
		types[col.Name] = col.Type
	}
	return types
}

func NewTable(schema []types.Column, rows int) *Table {
	// Initialize the Arrow memory allocator
	mem := memory.NewGoAllocator()

	// Create the map for Arrow builders and final arrays
	intBuilders := make(map[string]*array.Int32Builder)
	ints := make(map[string]*array.Int32)

	// Initialize a builder for each int column
	for _, col := range schema {
		if col.Type == types.IntType {
			intBuilders[col.Name] = array.NewInt32Builder(mem)
		}
	}

	return &Table{
		Schema:      schema,
		Types:       getColTypes(schema),
		Dim:         Dim{Rows: rows, Cols: len(schema)},
		Ints:        ints,
		Timestamps:  make(map[string][]time.Time),
		Bools:       make(map[string][]bool),
		IntBuilders: intBuilders,
		mem:         mem,
	}
}

func (t *Table) Append(col string, val any) error {
	switch t.Types[col] {
	case types.IntType:
		t.muInts.Lock()
		t.IntBuilders[col].Append(val.(int32))
		t.muInts.Unlock()
	case types.TimestampType:
		t.muTimestamps.Lock()
		t.Timestamps[col] = append(t.Timestamps[col], val.(time.Time))
		t.muTimestamps.Unlock()
	case types.BoolType:
		t.muBools.Lock()
		t.Bools[col] = append(t.Bools[col], val.(bool))
		t.muBools.Unlock()
	}
	return nil
}

func (t *Table) BuildInts() {
	t.muInts.Lock()
	defer t.muInts.Unlock()

	for colName, builder := range t.IntBuilders {
		// Build a new Int32 array from the builder
		t.Ints[colName] = builder.NewInt32Array()
		// Reset the builder for future use
		builder.Release()
		t.IntBuilders[colName] = array.NewInt32Builder(t.mem)
	}
}

func (t *Table) GetInts(col string) (*array.Int32, error) {
	t.muInts.Lock()
	defer t.muInts.Unlock()

	// This will get the already-built Arrow array
	if arr, ok := t.Ints[col]; ok {
		return arr, nil
	}
	// If the data is still in the builder, you might want to build it first.
	// For simplicity, we'll assume the BuildInts method is called before getting.
	return nil, nil // Or return an error if not found
}

func (t *Table) GetTimestamps(col string) ([]time.Time, error) {
	t.muTimestamps.Lock()
	defer t.muTimestamps.Unlock()
	return t.Timestamps[col], nil
}

func (t *Table) GetBools(col string) ([]bool, error) {
	t.muBools.Lock()
	defer t.muBools.Unlock()
	return t.Bools[col], nil
}

func (t *Table) SortInts() {
	t.muInts.Lock()
	defer t.muInts.Unlock()

	for _, col := range t.Schema {
		if col.Type == types.IntType {
			// Get the Go slice from the Arrow array
			slice := t.Ints[col.Name].Int32Values()
			// Sort the slice
			slices.Sort(slice)

			// Build a new Arrow array from the sorted slice
			newBuilder := array.NewInt32Builder(t.mem)
			// Corrected line: provide an empty []bool slice for valid values
			newBuilder.AppendValues(slice, nil) // Use nil to indicate all values are valid

			// Release the old array and assign the new one
			t.Ints[col.Name].Release()
			t.Ints[col.Name] = newBuilder.NewInt32Array()
			newBuilder.Release()
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

	// Release the Arrow arrays and builders
	for _, arr := range t.Ints {
		arr.Release()
	}
	for _, builder := range t.IntBuilders {
		builder.Release()
	}

	t.Ints = make(map[string]*array.Int32)
	t.IntBuilders = make(map[string]*array.Int32Builder)
	// Reinitialize builders
	for _, col := range t.Schema {
		if col.Type == types.IntType {
			t.IntBuilders[col.Name] = array.NewInt32Builder(t.mem)
		}
	}
	return nil
}

func CreateInt32Array(values []int32) *array.Int32 {
	mem := memory.NewGoAllocator()
	builder := array.NewInt32Builder(mem)
	defer builder.Release()

	builder.AppendValues(values, nil)
	return builder.NewInt32Array()
}
