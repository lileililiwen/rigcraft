//! Guards the `(s)` pseudo-plural out of every user-visible sentence.
//!
//! The service and the Web app must spell `0 units`, `1 unit`, `2 units`
//! rather than `2 unit(s)`. This scan is the rule that keeps the fix from
//! regressing: it fails, naming the file and line, if one comes back.

use std::fs;
use std::path::{Path, PathBuf};

/// True when `line` contains a word ending in the `(s)` pseudo-plural, which
/// is the same shape as the `\b[a-z]+\(s\)` search the requirement names.
fn pseudo_plural_at(line: &str) -> bool {
    let mut search = 0;
    while let Some(found) = line[search..].find("(s)") {
        let index = search + found;
        let mut start = index;
        while start > 0 && line.as_bytes()[start - 1].is_ascii_lowercase() {
            start -= 1;
        }
        if start < index {
            let boundary = line[..start]
                .chars()
                .next_back()
                .is_none_or(|before| !(before.is_alphanumeric() || before == '_'));
            if boundary {
                return true;
            }
        }
        search = index + 3;
    }
    false
}

fn collect(dir: &Path, extensions: &[&str], out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|error| panic!("read {dir:?}: {error}"));
    for entry in entries {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect(&path, extensions, out);
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extensions.contains(&extension))
        {
            out.push(path);
        }
    }
}

fn findings(dir: &str, extensions: &[&str]) -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
    let mut files = Vec::new();
    collect(&root, extensions, &mut files);
    assert!(!files.is_empty(), "no sources found under {dir}");
    let mut found = Vec::new();
    for file in files {
        let text = fs::read_to_string(&file).expect("source is UTF-8");
        for (number, line) in text.lines().enumerate() {
            if pseudo_plural_at(line) {
                found.push(format!(
                    "{}:{}: {}",
                    file.display(),
                    number + 1,
                    line.trim()
                ));
            }
        }
    }
    found
}

#[test]
fn no_pseudo_plural_in_service_or_web_sources() {
    let mut found = findings("src", &["rs"]);
    found.extend(findings("apps/web/src", &["ts", "tsx"]));
    assert!(
        found.is_empty(),
        "the `(s)` pseudo-plural is still user-visible:\n{}",
        found.join("\n")
    );
}

#[test]
fn the_detector_matches_a_pseudo_plural() {
    assert!(pseudo_plural_at("2 unit(s) remain"));
    assert!(pseudo_plural_at("with 1 conflict(s)"));
    assert!(!pseudo_plural_at("2 units remain"));
    assert!(!pseudo_plural_at("the (s) suffix"));
    assert!(!pseudo_plural_at("plural(count, singular, plural)"));
}

#[test]
fn helper_renders_zero_one_and_many() {
    assert_eq!(rigcraft::text::plural(0u64, "unit", "units"), "0 units");
    assert_eq!(rigcraft::text::plural(1u64, "unit", "units"), "1 unit");
    assert_eq!(rigcraft::text::plural(2u64, "unit", "units"), "2 units");
    assert_eq!(rigcraft::text::plural(0i64, "step", "steps"), "0 steps");
    assert_eq!(rigcraft::text::plural(1usize, "step", "steps"), "1 step");
    assert_eq!(
        rigcraft::text::plural(2.0f64, "module", "modules"),
        "2 modules"
    );
}
