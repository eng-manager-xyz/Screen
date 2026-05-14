//! `DataFrame` — column-oriented data table the Plot facade
//! consumes.
//!
//! A `DataFrame` is built once from a `Vec<R>` of arbitrary
//! user-defined rows + a row-flattener closure that extracts each
//! row's columns as `(name, Value)` pairs. After that, every
//! encoding refers to columns by name and runs against the
//! flattened representation.
//!
//! Trade-off: pay an O(n × cols) flatten cost on construction,
//! gain string-based encoding ergonomics + automatic
//! scale-derivation from column values.

use std::collections::HashMap;

/// One cell's value. Number-typed cells feed Linear / Log / Y
/// scales; Category-typed cells feed Band / Ordinal / Color
/// encodings.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// Numeric cell — feeds linear / log scales.
    Number(f32),
    /// Categorical / string cell — feeds band / ordinal scales.
    Category(String),
}

impl Value {
    /// Returns the numeric value if this is `Value::Number`,
    /// else `None`.
    #[must_use]
    pub fn as_number(&self) -> Option<f32> {
        match self {
            Value::Number(n) => Some(*n),
            Value::Category(_) => None,
        }
    }

    /// Returns the category string if this is `Value::Category`,
    /// else `None`.
    #[must_use]
    pub fn as_category(&self) -> Option<&str> {
        match self {
            Value::Category(s) => Some(s),
            Value::Number(_) => None,
        }
    }
}

/// Column-oriented data table.
///
/// # Example
///
/// ```
/// use wisp_chart::plot::{DataFrame, Value};
///
/// struct Sale { quarter: &'static str, revenue: f32 }
/// let rows = vec![
///     Sale { quarter: "Q1", revenue: 38.0 },
///     Sale { quarter: "Q2", revenue: 52.0 },
/// ];
/// let df = DataFrame::from_rows(&rows, |s| vec![
///     ("quarter".into(), Value::Category(s.quarter.into())),
///     ("revenue".into(), Value::Number(s.revenue)),
/// ]);
/// assert_eq!(df.row_count(), 2);
/// assert_eq!(df.column("quarter").unwrap().len(), 2);
/// ```
#[derive(Clone, Debug, Default)]
pub struct DataFrame {
    columns: HashMap<String, Vec<Value>>,
    column_order: Vec<String>,
    row_count: usize,
}

impl DataFrame {
    /// Build from a row iterable + a flattener closure.
    pub fn from_rows<R, F>(rows: &[R], flatten: F) -> Self
    where
        F: Fn(&R) -> Vec<(String, Value)>,
    {
        let mut columns: HashMap<String, Vec<Value>> = HashMap::new();
        let mut column_order: Vec<String> = Vec::new();
        for row in rows {
            for (name, value) in flatten(row) {
                if !columns.contains_key(&name) {
                    column_order.push(name.clone());
                }
                columns.entry(name).or_default().push(value);
            }
        }
        Self {
            columns,
            column_order,
            row_count: rows.len(),
        }
    }

    /// Total row count.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// Column names in insertion order.
    pub fn column_names(&self) -> impl Iterator<Item = &str> {
        self.column_order.iter().map(String::as_str)
    }

    /// Access a column by name.
    #[must_use]
    pub fn column(&self, name: &str) -> Option<&[Value]> {
        self.columns.get(name).map(Vec::as_slice)
    }

    /// Distinct category values in `name` in insertion order, or
    /// `None` if the column is absent or numeric.
    #[must_use]
    pub fn distinct_categories(&self, name: &str) -> Option<Vec<String>> {
        let col = self.column(name)?;
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for v in col {
            if let Some(s) = v.as_category()
                && seen.insert(s.to_owned())
            {
                out.push(s.to_owned());
            }
        }
        if out.is_empty() {
            return None;
        }
        Some(out)
    }

    /// (min, max) of `name` if it's a numeric column, else
    /// `None`. Returns `None` for an empty numeric column.
    #[must_use]
    pub fn numeric_extent(&self, name: &str) -> Option<(f32, f32)> {
        let col = self.column(name)?;
        let mut lo = f32::INFINITY;
        let mut hi = f32::NEG_INFINITY;
        let mut any = false;
        for v in col {
            if let Some(n) = v.as_number() {
                lo = lo.min(n);
                hi = hi.max(n);
                any = true;
            }
        }
        if any { Some((lo, hi)) } else { None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Sale {
        q: &'static str,
        r: f32,
    }

    fn fixture() -> DataFrame {
        let rows = vec![
            Sale { q: "Q1", r: 38.0 },
            Sale { q: "Q2", r: 52.0 },
            Sale { q: "Q3", r: 47.0 },
            Sale { q: "Q4", r: 64.0 },
        ];
        DataFrame::from_rows(&rows, |s| {
            vec![
                ("q".into(), Value::Category(s.q.into())),
                ("r".into(), Value::Number(s.r)),
            ]
        })
    }

    #[test]
    fn row_count_matches_input() {
        assert_eq!(fixture().row_count(), 4);
    }

    #[test]
    fn columns_preserve_insertion_order() {
        let df = fixture();
        let names: Vec<&str> = df.column_names().collect();
        assert_eq!(names, vec!["q", "r"]);
    }

    #[test]
    fn distinct_categories_dedupes() {
        let rows = vec![
            Sale { q: "A", r: 1.0 },
            Sale { q: "A", r: 2.0 },
            Sale { q: "B", r: 3.0 },
        ];
        let df = DataFrame::from_rows(&rows, |s| vec![("q".into(), Value::Category(s.q.into()))]);
        assert_eq!(
            df.distinct_categories("q"),
            Some(vec!["A".into(), "B".into()])
        );
    }

    #[test]
    fn numeric_extent_finds_min_max() {
        let df = fixture();
        assert_eq!(df.numeric_extent("r"), Some((38.0, 64.0)));
    }

    #[test]
    fn distinct_categories_on_numeric_column_is_none() {
        assert_eq!(fixture().distinct_categories("r"), None);
    }

    #[test]
    fn numeric_extent_on_categorical_column_is_none() {
        assert_eq!(fixture().numeric_extent("q"), None);
    }
}
