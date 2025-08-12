package table

import (
	"sort"
	"sync"
	"time"

	"github.com/phdah/sql-tdg/internals/types"

	"github.com/apache/arrow/go/v14/arrow/array"
	"github.com/apache/arrow/go/v14/arrow/memory"
	"slices"
)

// Dim represents the dimensions of a table, with rows and columns.
type Dim struct {
	Rows int
	Cols int
}

// Table represents a data table with columns of various types. It stores
// integer columns using Apache Arrow arrays for efficient batch processing,
// while timestamp and boolean columns are stored as standard Go slices.
type Table struct {
	Schema []types.Column
	Types  map[string]types.Type
	Dim    Dim

	Ints       map[string]*array.Int32
	Timestamps map[string][]time.Time
	Bools      map[string][]bool

	IntBuilders map[string]*array.Int32Builder
	mem         memory.Allocator

	muInts       sync.Mutex
	muTimestamps sync.Mutex
	muBools      sync.Mutex
}

// getColTypes returns a map from column names to their corresponding types
// based on the provided schema.
func getColTypes(schema []types.Column) map[string]types.Type {
	types := make(map[string]types.Type)
	for _, col := range schema {
		types[col.Name] = col.Type
	}
	return types
}

// NewTable creates a new Table with the given schema and number of rows.
// It initializes the Arrow memory allocator, builders for integer columns,
// and empty maps for timestamps and booleans.
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

// Append adds a value to the specified column in the table. The value must
// be of the appropriate Go type for the column's Arrow type. For integer
// columns the value is appended to the Arrow builder; for timestamp and
// boolean columns it is appended to the corresponding slice.
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

// BuildInts finalizes all integer columns by building Arrow arrays from
// the current builders. The builders are then released and reinitialized
// for future use.
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

// GetInts returns the Arrow array for the specified integer column. If
// the column has not been built yet, the method returns nil. The caller
// should not modify the returned array directly.
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

// GetTimestamps returns the slice of timestamp values for the specified
// column. The slice is protected by a mutex to ensure thread safety.
func (t *Table) GetTimestamps(col string) ([]time.Time, error) {
	t.muTimestamps.Lock()
	defer t.muTimestamps.Unlock()
	return t.Timestamps[col], nil
}

// GetBools returns the slice of boolean values for the specified column.
func (t *Table) GetBools(col string) ([]bool, error) {
	t.muBools.Lock()
	defer t.muBools.Unlock()
	return t.Bools[col], nil
}

// SortInts sorts all integer columns in the table in ascending order.
// It retrieves the underlying Go slice from the Arrow array, sorts it,
// and rebuilds the Arrow array with the sorted values.
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

// SortTimestamps sorts all timestamp columns in ascending order.
func (t *Table) SortTimestamps() {
	t.muTimestamps.Lock()
	defer t.muTimestamps.Unlock()
	for _, col := range t.Schema {
		if col.Type == types.TimestampType {
			slice := t.Timestamps[col.Name]
			sort.Slice(slice, func(i, j int) bool {
				return slice[i].Before(slice[j])
			})
		}
	}
}

// Wipe clears all data from the table, releasing Arrow resources and
// resetting builders for future use. It should be called when the table
// is no longer needed to free memory.
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
