//! A repository check for credential leakage, bound to `cargo test`.
//!
//! AGENTS.md: a check that can be switched off is not a check. This one reads
//! committed, repo-relative input and fails the suite, so no environment
//! variable can retire it.

use std::fs;
use std::path::{Path, PathBuf};

fn rust_sources(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

fn web_sources(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            web_sources(&path, found);
        } else if matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("ts" | "tsx")
        ) {
            found.push(path);
        }
    }
}

const SINKS: [&str; 7] = [
    "tracing::",
    "println!",
    "print!(",
    "eprintln!",
    "dbg!",
    "panic!",
    "todo!",
];

const SECRETS: [&str; 8] = [
    "api_key",
    "apiKey",
    "SecretKey",
    "expose_to_provider",
    "authorization",
    "Authorization",
    "Bearer",
    "bearer",
];

#[test]
fn no_logging_or_panic_line_can_carry_a_credential() {
    let mut sources = Vec::new();
    rust_sources(Path::new("src"), &mut sources);
    assert!(!sources.is_empty(), "no Rust source was scanned");
    let mut offenders = Vec::new();
    for path in sources {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let mut in_tests = false;
        for (index, line) in text.lines().enumerate() {
            if line.trim_start().starts_with("#[cfg(test)]") {
                in_tests = true;
            }
            if in_tests {
                continue;
            }
            let sink = SINKS.iter().any(|sink| line.contains(sink));
            if !sink {
                continue;
            }
            if let Some(secret) = SECRETS.iter().find(|secret| line.contains(*secret)) {
                offenders.push(format!(
                    "{}:{} contains `{secret}` on a logging or panic line: {}",
                    path.display(),
                    index + 1,
                    line.trim()
                ));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a credential could reach a log or a panic message:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn the_web_client_never_persists_a_key() {
    let mut sources = Vec::new();
    web_sources(Path::new("apps/web/src"), &mut sources);
    assert!(!sources.is_empty(), "no Web source was scanned");
    let forbidden = [
        "localStorage",
        "sessionStorage",
        "document.cookie",
        "indexedDB",
    ];
    let mut offenders = Vec::new();
    for path in sources {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            if let Some(needle) = forbidden.iter().find(|needle| line.contains(*needle)) {
                offenders.push(format!(
                    "{}:{} uses `{needle}`: {}",
                    path.display(),
                    index + 1,
                    line.trim()
                ));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "the key is session-only, so browser storage must not be used:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn the_ai_requests_are_not_serializable() {
    // A response echo is impossible only while the inbound request types have no
    // Serialize derive. This guards the declaration, not just today's output.
    let source = fs::read_to_string("src/ai/mod.rs").unwrap();
    for marker in ["pub struct ParseRequest", "pub struct ReviseRequest"] {
        let index = source.find(marker).expect("request struct is declared");
        let block_start = source[..index].rfind("#[derive").unwrap_or(index);
        let attributes = &source[block_start..index];
        assert!(
            !attributes.contains("Serialize"),
            "{marker} must not derive Serialize"
        );
    }
}
