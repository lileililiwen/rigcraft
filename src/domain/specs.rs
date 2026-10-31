use super::*;

struct NumericSpecRule {
    key: &'static str,
    unit: &'static str,
    code: &'static str,
}

const NUMERIC_SPEC_RULES: &[NumericSpecRule] = &[
    NumericSpecRule {
        key: "tdp",
        unit: "W",
        code: "tdp",
    },
    NumericSpecRule {
        key: "power_draw",
        unit: "W",
        code: "power_draw",
    },
    NumericSpecRule {
        key: "wattage",
        unit: "W",
        code: "wattage",
    },
    NumericSpecRule {
        key: "length",
        unit: "mm",
        code: "length",
    },
    NumericSpecRule {
        key: "height",
        unit: "mm",
        code: "height",
    },
    NumericSpecRule {
        key: "width",
        unit: "mm",
        code: "width",
    },
    NumericSpecRule {
        key: "depth",
        unit: "mm",
        code: "depth",
    },
    NumericSpecRule {
        key: "capacity",
        unit: "GB",
        code: "capacity",
    },
    NumericSpecRule {
        key: "vram",
        unit: "GB",
        code: "vram",
    },
    NumericSpecRule {
        key: "rpm",
        unit: "RPM",
        code: "rpm",
    },
    NumericSpecRule {
        key: "peak_power",
        unit: "W",
        code: "peak_power",
    },
    NumericSpecRule {
        key: "max_gpu_length",
        unit: "mm",
        code: "max_gpu_length",
    },
    NumericSpecRule {
        key: "max_cooler_height",
        unit: "mm",
        code: "max_cooler_height",
    },
    NumericSpecRule {
        key: "max_psu_length",
        unit: "mm",
        code: "max_psu_length",
    },
    NumericSpecRule {
        key: "radiator_size",
        unit: "mm",
        code: "radiator_size",
    },
    NumericSpecRule {
        key: "front_radiator_mm",
        unit: "mm",
        code: "front_radiator_mm",
    },
    NumericSpecRule {
        key: "max_memory",
        unit: "GB",
        code: "max_memory",
    },
    NumericSpecRule {
        // A user-provided, sourced performance proxy. It is never inferred and
        // an absent value removes the performance dimension instead of scoring 0.
        key: "perf_score",
        unit: "pt",
        code: "perf_score",
    },
];

const TEXT_SPEC_KEYS: &[&str] = &[
    "socket",
    "chipset",
    "memory_type",
    "form_factor",
    "interface",
    "connector",
    "cooler_socket",
    "pcie_version",
    "bios",
    "note",
    "igpu",
    "bundled_cooler",
    "bios_supports",
    "ram_slots",
    "modules",
    "supported_form_factors",
    "psu_form_factor",
    "psu_connectors",
    "storage_interface",
    "type",
];

fn normalize_spec_key(raw: &str) -> String {
    raw.trim().to_lowercase().replace([' ', '-'], "_")
}

fn split_number_and_unit(value: &str) -> Option<(f64, Option<String>)> {
    let trimmed = value.trim();
    let number_end = trimmed
        .char_indices()
        .find(|(_, ch)| !ch.is_ascii_digit() && *ch != '.')
        .map(|(index, _)| index)
        .unwrap_or(trimmed.len());
    if number_end == 0 {
        return None;
    }
    let number = trimmed[..number_end].parse::<f64>().ok()?;
    if !number.is_finite() || number < 0.0 {
        return None;
    }
    let unit = trimmed[number_end..].trim();
    Some((
        number,
        if unit.is_empty() {
            None
        } else {
            Some(unit.to_string())
        },
    ))
}

pub fn parse_specs(raw: &str, provenance: &Provenance) -> Result<Vec<Spec>, FieldError> {
    let mut specs = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for part in raw.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let Some((key_raw, value_raw)) = part.split_once('=') else {
            return Err(FieldError::new(
                "specs",
                "spec_format",
                format!("Specification `{part}` is not in key=value form."),
            )
            .suggest(
                "Separate specifications with semicolons, for example socket=AM5;length=320mm.",
            ));
        };
        let key = normalize_spec_key(key_raw);
        let value = value_raw.trim();
        if key.is_empty() || value.is_empty() {
            return Err(FieldError::new(
                "specs",
                "spec_format",
                format!("Specification `{part}` needs a key and a value."),
            ));
        }
        if !seen.insert(key.clone()) {
            return Err(FieldError::new(
                "specs",
                "spec_duplicate",
                format!("Specification `{key}` is listed more than once."),
            ));
        }
        if let Some(rule) = NUMERIC_SPEC_RULES.iter().find(|rule| rule.key == key) {
            let Some((number, unit)) = split_number_and_unit(value) else {
                return Err(FieldError::new(
                    "specs",
                    "spec_format",
                    format!("Specification `{key}` needs a number."),
                )
                .suggest(format!("Write {key}=numeric{}", rule.unit)));
            };
            let Some(unit) = unit else {
                return Err(FieldError::new(
                    "specs",
                    "spec_unit",
                    format!("Specification `{key}` needs a unit."),
                )
                .suggest(format!("Write {key}={number}{}", rule.unit)));
            };
            if !unit.eq_ignore_ascii_case(rule.unit) {
                return Err(FieldError::new(
                    "specs",
                    "spec_unit",
                    format!("Specification `{key}` expects unit {}.", rule.unit),
                )
                .suggest(format!("Write {key}={number}{}", rule.unit)));
            }
            specs.push(Spec {
                key: rule.code.to_string(),
                raw: part.to_string(),
                number: Some(number),
                text: None,
                unit: Some(rule.unit.to_string()),
                status: SpecStatus::Confirmed,
                provenance: provenance.clone(),
            });
        } else {
            let is_text = TEXT_SPEC_KEYS.iter().any(|known| *known == key);
            specs.push(Spec {
                key,
                raw: part.to_string(),
                number: None,
                text: Some(value.to_string()),
                unit: None,
                status: if is_text {
                    SpecStatus::Confirmed
                } else {
                    SpecStatus::Unknown
                },
                provenance: provenance.clone(),
            });
        }
    }
    Ok(specs)
}
