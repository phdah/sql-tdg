package table

import (
	"fmt"
	"slices"
	"sync"
	"time"

	"github.com/apache/arrow/go/v14/arrow"
	"github.com/apache/arrow/go/v14/arrow/array"
	"github.com/apache/arrow/go/v14/arrow/memory"
	"github.com/phdah/sql-tdg/internals/types"
)

// Dim represents the dimensions of a table, with rows and columns.
type Dim struct {
	Rows int
	Cols int
}

// Table represents a data table with Arrow-backed columns.
type Table struct {
	Schema []types.Column
	Types  map[string]types.Type
	Dim    Dim

	Ints       map[string]*array.Int32
	Timestamps map[string]*array.Timestamp
	Bools      map[string]*array.Boolean
	Strings    map[string]*array.String

	IntBuilders       map[string]*array.Int32Builder
	TimestampBuilders map[string]*array.TimestampBuilder
	BoolBuilders      map[string]*array.BooleanBuilder
	StringBuilders    map[string]*array.StringBuilder
	mem               memory.Allocator

	muInts       sync.Mutex
	muTimestamps sync.Mutex
	muBools      sync.Mutex
	muStrings    sync.Mutex
}

// getColTypes returns a map from column names to their corresponding types
// based on the provided schema.
func getColTypes(schema []types.Column) map[string]types.Type {
	columnTypes := make(map[string]types.Type, len(schema))
	for _, col := range schema {
		columnTypes[col.Name] = col.Type
	}
	return columnTypes
}

// NewTable creates a new Table with the given schema and number of rows.
func NewTable(schema []types.Column, rows int) *Table {
	mem := memory.NewGoAllocator()
	t := &Table{
		Schema:            schema,
		Types:             getColTypes(schema),
		Dim:               Dim{Rows: rows, Cols: len(schema)},
		Ints:              make(map[string]*array.Int32),
		Timestamps:        make(map[string]*array.Timestamp),
		Bools:             make(map[string]*array.Boolean),
		Strings:           make(map[string]*array.String),
		IntBuilders:       make(map[string]*array.Int32Builder),
		TimestampBuilders: make(map[string]*array.TimestampBuilder),
		BoolBuilders:      make(map[string]*array.BooleanBuilder),
		StringBuilders:    make(map[string]*array.StringBuilder),
		mem:               mem,
	}
	t.resetBuilders()
	return t
}

func (t *Table) resetBuilders() {
	for _, col := range t.Schema {
		switch col.Type {
		case types.IntType:
			t.Ints[col.Name] = nil
			t.IntBuilders[col.Name] = array.NewInt32Builder(t.mem)
		case types.TimestampType:
			t.Timestamps[col.Name] = nil
			t.TimestampBuilders[col.Name] = array.NewTimestampBuilder(
				t.mem,
				&arrow.TimestampType{Unit: arrow.Microsecond},
			)
		case types.BoolType:
			t.Bools[col.Name] = nil
			t.BoolBuilders[col.Name] = array.NewBooleanBuilder(t.mem)
		case types.StringType:
			t.Strings[col.Name] = nil
			t.StringBuilders[col.Name] = array.NewStringBuilder(t.mem)
		}
	}
}

// Append adds a value to the specified column. The value must match the
// column's declared type.
func (t *Table) Append(col string, val any) error {
	typ, ok := t.Types[col]
	if !ok {
		return fmt.Errorf("unknown column %q", col)
	}

	switch typ {
	case types.IntType:
		value, ok := val.(int32)
		if !ok {
			return fmt.Errorf("expected int32 for column %q, got %T", col, val)
		}
		t.muInts.Lock()
		defer t.muInts.Unlock()
		t.IntBuilders[col].Append(value)
	case types.TimestampType:
		value, ok := val.(time.Time)
		if !ok {
			return fmt.Errorf("expected time.Time for column %q, got %T", col, val)
		}
		t.muTimestamps.Lock()
		defer t.muTimestamps.Unlock()
		t.TimestampBuilders[col].Append(arrow.Timestamp(value.UnixMicro()))
	case types.BoolType:
		value, ok := val.(bool)
		if !ok {
			return fmt.Errorf("expected bool for column %q, got %T", col, val)
		}
		t.muBools.Lock()
		defer t.muBools.Unlock()
		t.BoolBuilders[col].Append(value)
	case types.StringType:
		value, ok := val.(string)
		if !ok {
			return fmt.Errorf("expected string for column %q, got %T", col, val)
		}
		t.muStrings.Lock()
		defer t.muStrings.Unlock()
		t.StringBuilders[col].Append(value)
	default:
		return fmt.Errorf("unsupported column type %q", typ)
	}
	return nil
}

// BuildInts finalizes all integer builders into Arrow arrays.
func (t *Table) BuildInts() {
	t.muInts.Lock()
	defer t.muInts.Unlock()

	for colName, builder := range t.IntBuilders {
		if arr := t.Ints[colName]; arr != nil {
			arr.Release()
		}
		t.Ints[colName] = builder.NewInt32Array()
		builder.Release()
		t.IntBuilders[colName] = array.NewInt32Builder(t.mem)
	}
}

// BuildTimestamps finalizes all timestamp builders into Arrow arrays.
func (t *Table) BuildTimestamps() {
	t.muTimestamps.Lock()
	defer t.muTimestamps.Unlock()

	for colName, builder := range t.TimestampBuilders {
		if arr := t.Timestamps[colName]; arr != nil {
			arr.Release()
		}
		t.Timestamps[colName] = builder.NewTimestampArray()
		builder.Release()
		t.TimestampBuilders[colName] = array.NewTimestampBuilder(
			t.mem,
			&arrow.TimestampType{Unit: arrow.Microsecond},
		)
	}
}

// BuildBools finalizes all boolean builders into Arrow arrays.
func (t *Table) BuildBools() {
	t.muBools.Lock()
	defer t.muBools.Unlock()

	for colName, builder := range t.BoolBuilders {
		if arr := t.Bools[colName]; arr != nil {
			arr.Release()
		}
		t.Bools[colName] = builder.NewBooleanArray()
		builder.Release()
		t.BoolBuilders[colName] = array.NewBooleanBuilder(t.mem)
	}
}

// BuildStrings finalizes all string builders into Arrow arrays.
func (t *Table) BuildStrings() {
	t.muStrings.Lock()
	defer t.muStrings.Unlock()

	for colName, builder := range t.StringBuilders {
		if arr := t.Strings[colName]; arr != nil {
			arr.Release()
		}
		t.Strings[colName] = builder.NewStringArray()
		builder.Release()
		t.StringBuilders[colName] = array.NewStringBuilder(t.mem)
	}
}

// GetInts returns a copy of the integer values for the specified column.
// It returns nil when the column exists but has not been built yet.
func (t *Table) GetInts(col string) ([]int32, error) {
	t.muInts.Lock()
	defer t.muInts.Unlock()

	arr, ok := t.Ints[col]
	if !ok {
		return nil, fmt.Errorf("column not found or not an integer column: %s", col)
	}
	if arr == nil {
		return nil, nil
	}
	return append([]int32(nil), arr.Int32Values()...), nil
}

// GetAllInts returns all integer columns as Go slices.
func (t *Table) GetAllInts() (map[string][]int32, error) {
	result := make(map[string][]int32)
	for _, col := range t.Schema {
		if col.Type != types.IntType {
			continue
		}
		values, err := t.GetInts(col.Name)
		if err != nil {
			return nil, err
		}
		result[col.Name] = values
	}
	return result, nil
}

// GetTimestamps returns the timestamp values for the specified column.
// It returns nil when the column exists but has not been built yet.
func (t *Table) GetTimestamps(col string) ([]time.Time, error) {
	t.muTimestamps.Lock()
	defer t.muTimestamps.Unlock()

	arr, ok := t.Timestamps[col]
	if !ok {
		return nil, fmt.Errorf("column not found or not a timestamp column: %s", col)
	}
	if arr == nil {
		return nil, nil
	}
	result := make([]time.Time, arr.Len())
	for i := range arr.Len() {
		result[i] = arr.Value(i).ToTime(arrow.Microsecond)
	}
	return result, nil
}

// GetAllTimestamps returns all timestamp columns as Go slices.
func (t *Table) GetAllTimestamps() (map[string][]time.Time, error) {
	result := make(map[string][]time.Time)
	for _, col := range t.Schema {
		if col.Type != types.TimestampType {
			continue
		}
		values, err := t.GetTimestamps(col.Name)
		if err != nil {
			return nil, err
		}
		result[col.Name] = values
	}
	return result, nil
}

// GetBools returns the boolean values for the specified column.
// It returns nil when the column exists but has not been built yet.
func (t *Table) GetBools(col string) ([]bool, error) {
	t.muBools.Lock()
	defer t.muBools.Unlock()

	arr, ok := t.Bools[col]
	if !ok {
		return nil, fmt.Errorf("column not found or not a boolean column: %s", col)
	}
	if arr == nil {
		return nil, nil
	}
	result := make([]bool, arr.Len())
	for i := range arr.Len() {
		result[i] = arr.Value(i)
	}
	return result, nil
}

// GetAllBools returns all boolean columns as Go slices.
func (t *Table) GetAllBools() (map[string][]bool, error) {
	result := make(map[string][]bool)
	for _, col := range t.Schema {
		if col.Type != types.BoolType {
			continue
		}
		values, err := t.GetBools(col.Name)
		if err != nil {
			return nil, err
		}
		result[col.Name] = values
	}
	return result, nil
}

// GetStrings returns the string values for the specified column.
// It returns nil when the column exists but has not been built yet.
func (t *Table) GetStrings(col string) ([]string, error) {
	t.muStrings.Lock()
	defer t.muStrings.Unlock()

	arr, ok := t.Strings[col]
	if !ok {
		return nil, fmt.Errorf("column not found or not a string column: %s", col)
	}
	if arr == nil {
		return nil, nil
	}
	result := make([]string, arr.Len())
	for i := range arr.Len() {
		result[i] = arr.Value(i)
	}
	return result, nil
}

// GetAllStrings returns all string columns as Go slices.
func (t *Table) GetAllStrings() (map[string][]string, error) {
	result := make(map[string][]string)
	for _, col := range t.Schema {
		if col.Type != types.StringType {
			continue
		}
		values, err := t.GetStrings(col.Name)
		if err != nil {
			return nil, err
		}
		result[col.Name] = values
	}
	return result, nil
}

// SortInts sorts all built integer columns in ascending order.
func (t *Table) SortInts() {
	t.muInts.Lock()
	defer t.muInts.Unlock()

	for _, col := range t.Schema {
		if col.Type != types.IntType {
			continue
		}
		arr := t.Ints[col.Name]
		if arr == nil {
			continue
		}
		values := append([]int32(nil), arr.Int32Values()...)
		slices.Sort(values)

		builder := array.NewInt32Builder(t.mem)
		builder.AppendValues(values, nil)
		newArr := builder.NewInt32Array()
		builder.Release()

		arr.Release()
		t.Ints[col.Name] = newArr
	}
}

// SortTimestamps sorts all built timestamp columns in ascending order.
func (t *Table) SortTimestamps() {
	t.muTimestamps.Lock()
	defer t.muTimestamps.Unlock()

	for _, col := range t.Schema {
		if col.Type != types.TimestampType {
			continue
		}
		arr := t.Timestamps[col.Name]
		if arr == nil {
			continue
		}
		values := append([]arrow.Timestamp(nil), arr.TimestampValues()...)
		slices.Sort(values)

		builder := array.NewTimestampBuilder(t.mem, &arrow.TimestampType{Unit: arrow.Microsecond})
		builder.AppendValues(values, nil)
		newArr := builder.NewTimestampArray()
		builder.Release()

		arr.Release()
		t.Timestamps[col.Name] = newArr
	}
}

// Wipe clears all built data and resets Arrow builders for future use.
func (t *Table) Wipe() error {
	t.muInts.Lock()
	defer t.muInts.Unlock()
	t.muTimestamps.Lock()
	defer t.muTimestamps.Unlock()
	t.muBools.Lock()
	defer t.muBools.Unlock()
	t.muStrings.Lock()
	defer t.muStrings.Unlock()

	for _, arr := range t.Ints {
		if arr != nil {
			arr.Release()
		}
	}
	for _, builder := range t.IntBuilders {
		if builder != nil {
			builder.Release()
		}
	}
	for _, arr := range t.Timestamps {
		if arr != nil {
			arr.Release()
		}
	}
	for _, builder := range t.TimestampBuilders {
		if builder != nil {
			builder.Release()
		}
	}
	for _, arr := range t.Bools {
		if arr != nil {
			arr.Release()
		}
	}
	for _, builder := range t.BoolBuilders {
		if builder != nil {
			builder.Release()
		}
	}
	for _, arr := range t.Strings {
		if arr != nil {
			arr.Release()
		}
	}
	for _, builder := range t.StringBuilders {
		if builder != nil {
			builder.Release()
		}
	}

	t.Ints = make(map[string]*array.Int32)
	t.Timestamps = make(map[string]*array.Timestamp)
	t.Bools = make(map[string]*array.Boolean)
	t.Strings = make(map[string]*array.String)
	t.IntBuilders = make(map[string]*array.Int32Builder)
	t.TimestampBuilders = make(map[string]*array.TimestampBuilder)
	t.BoolBuilders = make(map[string]*array.BooleanBuilder)
	t.StringBuilders = make(map[string]*array.StringBuilder)
	t.resetBuilders()
	return nil
}
