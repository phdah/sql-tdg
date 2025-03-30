package solver

import (
	"fmt"
	"math/rand"
	"sync"

	"github.com/phdah/sql-tdg/internals/table"
	"github.com/phdah/sql-tdg/internals/types"
)

type Generator struct {
	Columns []types.Column
}

func (g *Generator) generateColumn(domain types.Domain, col *types.Column, table *table.Table, rng *rand.Rand) {
	var err error
	for _, c := range col.Constraints {
		err = c.Apply(domain)
		if err != nil {
			panic(err)
		}
	}
	value, err := domain.RandomValue(rng)
	if err != nil {
		panic(err)
	}
	err = table.Append(col.Name, value)
	if err != nil {
		panic(err)
	}
}

func (g *Generator) Generate(table *table.Table, seed int64) {
	rng := rand.New(rand.NewSource(seed))
	var wg sync.WaitGroup
	workers := 4
	if table.Dim.Rows%workers != 0 {
		panic(
			fmt.Sprintf("number of rows (%v) not dividable with number of workers (%v)",
				table.Dim.Rows,
				workers,
			),
		)
	}
	valueSplit := table.Dim.Rows / workers

	wg.Add(workers)
	for range workers {
		go func() {
			defer wg.Done()
			for range valueSplit {
				for _, col := range table.Schema {
					switch col.Type {
					case types.IntType:
						domain := NewIntDomain()
						g.generateColumn(domain, &col, table, rng)
					case types.TimestampType:
						domain := NewTimestampDomain()
						g.generateColumn(domain, &col, table, rng)
					}
				}
			}

		}()
	}

	wg.Wait()
}
