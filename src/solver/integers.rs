//! Integer domain narrowing.

use crate::{IntConstraint, Interval};

use super::SolverError;

const DEFAULT_MIN: i32 = -1_000_000;
const DEFAULT_MAX: i32 = 1_000_000;

/// Allowed integer values represented as inclusive, non-overlapping intervals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntDomain {
    intervals: Vec<Interval>,
    total_min: i32,
    total_max: i32,
}

impl IntDomain {
    /// Creates the default integer domain.
    pub fn new() -> Self {
        Self::with_bounds(DEFAULT_MIN, DEFAULT_MAX)
            .expect("default integer bounds form a valid non-empty domain")
    }

    /// Creates an integer domain with inclusive bounds.
    pub fn with_bounds(min: i32, max: i32) -> Result<Self, SolverError> {
        let interval = Interval::new(min, max).map_err(|_| SolverError::UnsatisfiableDomain)?;
        Ok(Self {
            intervals: vec![interval],
            total_min: min,
            total_max: max,
        })
    }

    /// Returns the allowed intervals in ascending order.
    pub fn intervals(&self) -> &[Interval] {
        &self.intervals
    }

    /// Returns the smallest currently allowed value.
    pub const fn total_min(&self) -> i32 {
        self.total_min
    }

    /// Returns the largest currently allowed value.
    pub const fn total_max(&self) -> i32 {
        self.total_max
    }

    /// Applies one integer constraint to the domain.
    pub fn apply(&mut self, constraint: IntConstraint) -> Result<(), SolverError> {
        match constraint {
            IntConstraint::Equal(value) => self.retain_range(value, value),
            IntConstraint::NotEqual(value) => self.exclude(value),
            IntConstraint::LessThan(value) => {
                let Some(max) = value.checked_sub(1) else {
                    return Err(SolverError::UnsatisfiableDomain);
                };
                self.retain_range(self.total_min, max)
            }
            IntConstraint::LessThanOrEqual(value) => self.retain_range(self.total_min, value),
            IntConstraint::GreaterThan(value) => {
                let Some(min) = value.checked_add(1) else {
                    return Err(SolverError::UnsatisfiableDomain);
                };
                self.retain_range(min, self.total_max)
            }
            IntConstraint::GreaterThanOrEqual(value) => self.retain_range(value, self.total_max),
        }
    }

    /// Returns the number of currently allowed values.
    pub fn value_count(&self) -> usize {
        self.intervals
            .iter()
            .map(|interval| {
                let width = i64::from(interval.max()) - i64::from(interval.min()) + 1;
                usize::try_from(width).expect("valid interval width fits usize")
            })
            .sum()
    }

    /// Returns the allowed value at the zero-based position across all intervals.
    ///
    /// A generator can choose the position with an injected random source without coupling
    /// randomness to the domain itself.
    pub fn value_at(&self, index: usize) -> Result<i32, SolverError> {
        let value_count = self.value_count();
        if index >= value_count {
            return Err(SolverError::ValueIndexOutOfRange { index, value_count });
        }

        let mut remaining = index;
        for interval in &self.intervals {
            let width_i64 = i64::from(interval.max()) - i64::from(interval.min()) + 1;
            let width = usize::try_from(width_i64).expect("valid interval width fits usize");
            if remaining < width {
                let offset = i32::try_from(remaining).expect("domain offset fits i32");
                return Ok(interval.min() + offset);
            }
            remaining -= width;
        }

        Err(SolverError::ValueIndexOutOfRange { index, value_count })
    }

    fn retain_range(&mut self, min: i32, max: i32) -> Result<(), SolverError> {
        if min > max {
            return Err(SolverError::UnsatisfiableDomain);
        }

        let mut updated = Vec::with_capacity(self.intervals.len());
        for interval in &self.intervals {
            let narrowed_min = interval.min().max(min);
            let narrowed_max = interval.max().min(max);
            if narrowed_min <= narrowed_max {
                let narrowed = Interval::new(narrowed_min, narrowed_max)
                    .map_err(|_| SolverError::UnsatisfiableDomain)?;
                updated.push(narrowed);
            }
        }

        self.replace_intervals(updated)
    }

    fn exclude(&mut self, value: i32) -> Result<(), SolverError> {
        let mut updated = Vec::with_capacity(self.intervals.len() + 1);

        for interval in &self.intervals {
            if !interval.contains(value) {
                updated.push(*interval);
                continue;
            }

            if interval.min() < value {
                let max = value
                    .checked_sub(1)
                    .ok_or(SolverError::UnsatisfiableDomain)?;
                updated.push(
                    Interval::new(interval.min(), max)
                        .map_err(|_| SolverError::UnsatisfiableDomain)?,
                );
            }
            if value < interval.max() {
                let min = value
                    .checked_add(1)
                    .ok_or(SolverError::UnsatisfiableDomain)?;
                updated.push(
                    Interval::new(min, interval.max())
                        .map_err(|_| SolverError::UnsatisfiableDomain)?,
                );
            }
        }

        self.replace_intervals(updated)
    }

    fn replace_intervals(&mut self, intervals: Vec<Interval>) -> Result<(), SolverError> {
        let Some(first) = intervals.first() else {
            return Err(SolverError::UnsatisfiableDomain);
        };
        let Some(last) = intervals.last() else {
            return Err(SolverError::UnsatisfiableDomain);
        };

        self.total_min = first.min();
        self.total_max = last.max();
        self.intervals = intervals;
        Ok(())
    }
}

impl Default for IntDomain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::IntConstraint;

    use super::{IntDomain, SolverError};

    #[test]
    fn single_apply_matches_supported_operators() {
        let cases = [
            (
                IntConstraint::Equal(3),
                vec![(3, 3)],
                3,
                3,
            ),
            (
                IntConstraint::NotEqual(3),
                vec![(-1_000_000, 2), (4, 1_000_000)],
                -1_000_000,
                1_000_000,
            ),
            (
                IntConstraint::LessThan(3),
                vec![(-1_000_000, 2)],
                -1_000_000,
                2,
            ),
            (
                IntConstraint::LessThanOrEqual(3),
                vec![(-1_000_000, 3)],
                -1_000_000,
                3,
            ),
            (
                IntConstraint::GreaterThan(3),
                vec![(4, 1_000_000)],
                4,
                1_000_000,
            ),
            (
                IntConstraint::GreaterThanOrEqual(3),
                vec![(3, 1_000_000)],
                3,
                1_000_000,
            ),
        ];

        for (constraint, expected_intervals, expected_min, expected_max) in cases {
            let mut domain = IntDomain::new();
            domain.apply(constraint).expect("constraint should apply");

            let actual_intervals: Vec<_> = domain
                .intervals()
                .iter()
                .map(|interval| (interval.min(), interval.max()))
                .collect();
            assert_eq!(actual_intervals, expected_intervals);
            assert_eq!(domain.total_min(), expected_min);
            assert_eq!(domain.total_max(), expected_max);
        }
    }

    #[test]
    fn multi_apply_keeps_only_values_matching_every_constraint() {
        let mut domain = IntDomain::new();
        for constraint in [
            IntConstraint::Equal(3),
            IntConstraint::GreaterThan(-10),
            IntConstraint::GreaterThanOrEqual(0),
            IntConstraint::LessThan(200),
            IntConstraint::LessThanOrEqual(150),
            IntConstraint::NotEqual(100),
        ] {
            domain.apply(constraint).expect("constraints should overlap");
        }

        assert_eq!(domain.intervals().len(), 1);
        assert_eq!(domain.intervals()[0].min(), 3);
        assert_eq!(domain.intervals()[0].max(), 3);
        assert_eq!(domain.total_min(), 3);
        assert_eq!(domain.total_max(), 3);
    }

    #[test]
    fn contradictory_constraints_are_rejected() {
        let mut domain = IntDomain::new();
        domain
            .apply(IntConstraint::NotEqual(5))
            .expect("excluding one value should succeed");

        let error = domain
            .apply(IntConstraint::Equal(5))
            .expect_err("excluded value must not be restored");

        assert_eq!(error, SolverError::UnsatisfiableDomain);
    }

    #[test]
    fn value_at_exposes_domain_without_owning_randomness() {
        let mut domain = IntDomain::with_bounds(1, 5).expect("valid bounds");
        domain
            .apply(IntConstraint::NotEqual(3))
            .expect("excluding one value should succeed");

        let values = (0..domain.value_count())
            .map(|index| domain.value_at(index).expect("index should be valid"))
            .collect::<Vec<_>>();

        assert_eq!(values, vec![1, 2, 4, 5]);
        assert_eq!(
            domain.value_at(4),
            Err(SolverError::ValueIndexOutOfRange {
                index: 4,
                value_count: 4,
            })
        );
    }
}
