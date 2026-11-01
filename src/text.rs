//! One pluralisation helper for every count the service prints.
//!
//! Zero is plural in English, so the singular form is used only for exactly
//! one: `0 units`, `1 unit`, `2 units`.

use std::fmt::Display;

/// Render a count with its noun, choosing the singular form only for one.
///
/// The count type stays whatever the caller already has; integers, unsigned
/// counts and the `f64` slot arithmetic all print the same way.
pub fn plural<T>(count: T, singular: &str, plural_form: &str) -> String
where
    T: Display + PartialEq + Copy + From<u8>,
{
    let noun = if count == T::from(1u8) {
        singular
    } else {
        plural_form
    };
    format!("{count} {noun}")
}

#[cfg(test)]
mod tests {
    use super::plural;

    #[test]
    fn zero_is_plural() {
        assert_eq!(plural(0u64, "unit", "units"), "0 units");
    }

    #[test]
    fn one_is_singular() {
        assert_eq!(plural(1u64, "unit", "units"), "1 unit");
    }

    #[test]
    fn many_is_plural() {
        assert_eq!(plural(2u64, "unit", "units"), "2 units");
    }

    #[test]
    fn fractional_counts_print_without_a_trailing_zero() {
        assert_eq!(plural(2.0f64, "module", "modules"), "2 modules");
        assert_eq!(plural(1.0f64, "module", "modules"), "1 module");
    }
}
