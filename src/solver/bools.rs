//! Boolean domain narrowing.

use crate::BoolConstraint;

use super::SolverError;

/// Allowed boolean value for a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BoolDomain {
    required: Option<bool>,
}

impl BoolDomain {
    /// Creates an unconstrained boolean domain.
    pub const fn new() -> Self {
        Self { required: None }
    }

    /// Applies one boolean constraint to the domain.
    pub fn apply(&mut self, constraint: BoolConstraint) -> Result<(), SolverError> {
        let requested = match constraint {
            BoolConstraint::IsTrue => true,
            BoolConstraint::IsFalse => false,
        };

        if let Some(existing) = self.required {
            if existing != requested {
                return Err(SolverError::UnsatisfiableDomain);
            }
        }

        self.required = Some(requested);
        Ok(())
    }

    /// Returns whether a constraint has fixed the domain to one value.
    pub const fn is_constrained(&self) -> bool {
        self.required.is_some()
    }

    /// Returns the fixed value when constrained.
    pub const fn required_value(&self) -> Option<bool> {
        self.required
    }

    /// Returns the value exposed for generation.
    ///
    /// An unconstrained domain yields `true` so generation always has an allowed value.
    pub const fn value(&self) -> bool {
        match self.required {
            Some(value) => value,
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::BoolConstraint;

    use super::{BoolDomain, SolverError};

    #[test]
    fn single_apply_sets_true_or_false() {
        let mut true_domain = BoolDomain::new();
        true_domain
            .apply(BoolConstraint::IsTrue)
            .expect("true should be allowed");
        assert_eq!(true_domain.required_value(), Some(true));

        let mut false_domain = BoolDomain::new();
        false_domain
            .apply(BoolConstraint::IsFalse)
            .expect("false should be allowed");
        assert_eq!(false_domain.required_value(), Some(false));
    }

    #[test]
    fn repeated_matching_constraints_are_allowed() {
        for constraint in [BoolConstraint::IsTrue, BoolConstraint::IsFalse] {
            let mut domain = BoolDomain::new();
            domain
                .apply(constraint)
                .expect("first constraint should be allowed");
            domain
                .apply(constraint)
                .expect("matching constraint should remain allowed");
        }
    }

    #[test]
    fn contradictory_constraints_are_rejected() {
        let mut domain = BoolDomain::new();
        domain
            .apply(BoolConstraint::IsFalse)
            .expect("false should be allowed initially");

        let error = domain
            .apply(BoolConstraint::IsTrue)
            .expect_err("opposite boolean requirement must fail");

        assert_eq!(error, SolverError::UnsatisfiableDomain);
        assert_eq!(domain.required_value(), Some(false));
    }
}
