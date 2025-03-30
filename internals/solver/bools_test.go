package solver_test

import (
	"fmt"
	"testing"

	"github.com/phdah/sql-tdg/internals/solver"
	"github.com/phdah/sql-tdg/internals/types"
	"github.com/stretchr/testify/require"
)

func TestBool_Single_Apply(t *testing.T) {
	tests := []struct {
		name string // description of this test case
		// Named input parameters for target function.
		domain     types.Domain
		want       types.Domain
		wantErr    error
		conditions []types.Constraints
	}{
		{
			name:   "set one true",
			domain: solver.NewBoolDomain(),
			want: &solver.BoolDomain{
				Condition:      true,
				HasBeenChanged: true,
			},
			conditions: []types.Constraints{
				solver.BoolTrue{},
			},
			wantErr: nil,
		},
		{
			name:   "set one false",
			domain: solver.NewBoolDomain(),
			want: &solver.BoolDomain{
				Condition:      false,
				HasBeenChanged: true,
			},
			conditions: []types.Constraints{
				solver.BoolFalse{},
			},
			wantErr: nil,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := require.New(t)
			var err error
			for _, c := range tt.conditions {
				err = c.Apply(tt.domain)
				if tt.wantErr != nil && err != nil {
					r.Error(err)
					r.Contains(tt.wantErr.Error(), err.Error())
					return
				}
				r.NoErrorf(err, fmt.Sprintf("Error was rasied: %v", err))
			}
			r.Equal(tt.want, tt.domain)
		})
	}
}

func TestBool_Multi_Apply(t *testing.T) {
	tests := []struct {
		name string // description of this test case
		// Named input parameters for target function.
		domain     types.Domain
		want       types.Domain
		wantErr    error
		conditions []types.Constraints
	}{
		{
			name:   "set multiple true",
			domain: solver.NewBoolDomain(),
			want: &solver.BoolDomain{
				Condition:      true,
				HasBeenChanged: true,
			},
			conditions: []types.Constraints{
				solver.BoolTrue{},
				solver.BoolTrue{},
			},
			wantErr: nil,
		},
		{
			name:   "set multiple false",
			domain: solver.NewBoolDomain(),
			want: &solver.BoolDomain{
				Condition:      false,
				HasBeenChanged: true,
			},
			conditions: []types.Constraints{
				solver.BoolFalse{},
				solver.BoolFalse{},
			},
			wantErr: nil,
		},
		{
			name:   "not allowed condition error",
			domain: solver.NewBoolDomain(),
			want: &solver.BoolDomain{
				Condition:      false,
				HasBeenChanged: true,
			},
			conditions: []types.Constraints{
				solver.BoolFalse{},
				solver.BoolTrue{},
			},
			wantErr: fmt.Errorf("BoolDomain Condition has already been updated to false"),
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := require.New(t)
			var err error
			for _, c := range tt.conditions {
				err = c.Apply(tt.domain)
				if tt.wantErr != nil && err != nil {
					r.Error(err)
					r.Contains(tt.wantErr.Error(), err.Error())
					return
				}
				r.NoErrorf(err, fmt.Sprintf("Error was rasied: %v", err))
			}
			r.Equal(tt.want, tt.domain)
		})
	}
}
