package solver

import (
	"sync"

	"github.com/phdah/sql-tdg/internals/table"
	"github.com/phdah/sql-tdg/internals/types"
)

type Generator struct {
	Columns []types.Column
}

func (g *Generator) Generate(table *table.Table) {
	var wg sync.WaitGroup
	workers := 1 // Set to 1 for debugging

	wg.Add(workers)
	for range workers {
		go func() {
			defer wg.Done()
			for range table.Dim.Rows / workers {
				for _, col := range table.Schema {
					switch col.Type {
					case types.IntType:
						domain := IntDomain{Min: -1_000_000, Max: 1_000_000}
						var applied types.Domain
						var err error
						for _, c := range col.Constraints {
							applied, err = c.Apply(domain)
							if err != nil {
								panic(err)
							}
							domain = applied.(IntDomain)
						}
						value := domain.RandomValue()
						err = table.Append(col.Name, value)
						if err != nil {
							panic(err)
						}
					}
				}
			}

		}()
	}

	wg.Wait()
}
