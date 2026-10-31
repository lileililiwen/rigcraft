//! rigcraft: rust-web scaffold (deterministic asset 0.1.0).

/// Stable greeting used by native tests.
pub fn greeting() -> &'static str {
    "hello from rigcraft"
}

fn main() {
    println!("{}", greeting());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greeting_is_stable() {
        assert_eq!(greeting(), "hello from rigcraft");
    }
}
