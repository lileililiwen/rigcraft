//! UTF-8 CSV preview parsing with explicit mapping and per-field errors.

use std::collections::BTreeMap;
use std::path::Path;

use crate::domain::{
    csv_checksum, evaluate_row, expires_at, normalize_sku, now_rfc3339, reject_oversize,
    DuplicateGroup, Preview, PreviewRow, PreviewSummary, RowStatus, RowValues,
};
use crate::error::ApiError;

pub const LOGICAL_FIELDS: &[&str] = &[
    "sku",
    "category",
    "model",
    "quantity",
    "price",
    "currency",
    "specs",
    "cost",
    "source",
    "sourceUrl",
];

pub const REQUIRED_FIELDS: &[&str] = &["sku", "category", "model", "quantity", "price"];

/// One physically buildable AM5 machine. Every compatibility fact was checked
/// against the manufacturer page in `sourceUrl` on 2026-10-31; facts those pages
/// do not state (BIOS support, peak power) are omitted so they stay unknown.
/// Quantity and price are operator-entered sample stock values, not market data.
pub const SAMPLE_CSV: &str = "sku,category,model,quantity,price,currency,specs,source,sourceUrl\n\
CPU-7800X3D,cpu,AMD Ryzen 7 7800X3D,1,449.00,USD,socket=AM5;tdp=120W;igpu=yes,catalog:manufacturer@2026-10-31,https://www.amd.com/en/products/processors/desktops/ryzen/7000-series/amd-ryzen-7-7800x3d.html\n\
MB-B650E-F,motherboard,ASUS ROG STRIX B650E-F GAMING WIFI,1,229.00,USD,socket=AM5;form_factor=ATX;memory_type=DDR5;ram_slots=4;max_memory=192GB;pcie_version=PCIe 5.0;storage_interface=PCIe 4.0 x4,catalog:manufacturer@2026-10-31,https://rog.asus.com/motherboards/rog-strix/rog-strix-b650e-f-gaming-wifi-model/spec/\n\
RAM-FURY-32-6000,ram,Kingston FURY Beast 32GB (2x16GB) DDR5-6000 CL36,1,124.00,USD,memory_type=DDR5;capacity=32GB;modules=2,catalog:manufacturer@2026-10-31,https://www.kingston.com/dataSheets/KF560C36BBEK2-32.pdf\n\
GPU-4070-FE,gpu,NVIDIA GeForce RTX 4070 Founders Edition,1,549.00,USD,length=244mm;interface=PCIe 4.0 x16;tdp=200W;vram=12GB;connector=2x8pin,catalog:manufacturer@2026-10-31,https://www.nvidia.com/en-us/geforce/graphics-cards/40-series/rtx-4070-4070ti/\n\
PSU-RM750E,psu,Corsair RM750e,1,99.00,USD,wattage=750W;form_factor=ATX;length=140mm;psu_connectors=1x16pin+3x8pin,catalog:manufacturer@2026-10-31,https://www.corsair.com/us/en/p/psu/cp-9020262-na/rm750e-fully-modular-low-noise-atx-power-supply-cp-9020262-na\n\
CASE-4000D,case,Corsair 4000D AIRFLOW Tempered Glass,1,94.00,USD,supported_form_factors=E-ATX+ATX+micro-ATX+Mini-ITX;max_gpu_length=360mm;max_cooler_height=170mm;max_psu_length=220mm;psu_form_factor=ATX,catalog:manufacturer@2026-10-31,https://www.corsair.com/us/en/p/case/cc-9011200-ww/4000d-airflow-tempered-glass-mid-tower-atx-case-cc-9011200-ww\n\
COOL-PA120SE,cooler,Thermalright Peerless Assassin 120 SE,1,39.90,USD,height=155mm;cooler_socket=AM5;type=air,catalog:manufacturer@2026-10-31,https://www.thermalright.com/product/peerless-assassin-120-se/\n\
SSD-MP600-1TB,storage,Corsair MP600 PRO NH 1TB,1,89.00,USD,interface=PCIe 4.0 x4;capacity=1000GB;form_factor=M.2 2280,catalog:manufacturer@2026-10-31,https://www.corsair.com/us/en/p/data-storage/cssd-f1000gbmp600pnh/mp600-pro-nh-1tb-pcie-gen4-x4-nvme-m-2-ssd-cssd-f1000gbmp600pnh\n\
SSD-MP600-2TB,storage,Corsair MP600 PRO NH 2TB,1,149.00,USD,interface=PCIe 4.0 x4;capacity=2000GB;form_factor=M.2 2280,catalog:manufacturer@2026-10-31,https://www.corsair.com/us/en/p/data-storage/cssd-f2000gbmp600pnh/mp600-pro-nh-2tb-pcie-gen4-x4-nvme-m-2-ssd-cssd-f2000gbmp600pnh\n\
SSD-MP600-4TB,storage,Corsair MP600 PRO NH 4TB,1,299.00,USD,interface=PCIe 4.0 x4;capacity=4000GB;form_factor=M.2 2280,catalog:manufacturer@2026-10-31,https://www.corsair.com/us/en/p/data-storage/cssd-f4000gbmp600pnh/mp600-pro-nh-4tb-pcie-gen4-x4-nvme-m-2-ssd-cssd-f4000gbmp600pnh\n";

#[derive(Debug, Clone)]
pub struct ParsedCsv {
    pub preview: Preview,
    pub sha256: String,
}

fn canonical_header(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

fn aliases_for(logical: &str) -> &'static [&'static str] {
    match logical {
        "sku" => &["sku", "id", "code", "item", "itemcode"],
        "category" => &["category", "type", "kind"],
        "model" => &["model", "name", "product", "productname", "description"],
        "quantity" => &["quantity", "qty", "stock", "onhand", "available"],
        "price" => &["price", "saleprice", "unitprice", "sellingprice"],
        "currency" => &["currency", "ccy"],
        "specs" => &["specs", "specifications", "spec"],
        "cost" => &["cost", "purchaseprice", "buyprice"],
        "source" => &["source", "datasource", "provenancesource"],
        "sourceUrl" => &["sourceurl", "provenanceurl"],
        _ => &[],
    }
}

fn safe_filename(raw: &str) -> String {
    let base = Path::new(raw)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("inventory.csv")
        .trim();
    if base.is_empty() {
        "inventory.csv".to_string()
    } else {
        base.chars().take(200).collect()
    }
}

fn resolve_mapping(
    headers: &[String],
    requested: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, ApiError> {
    let mut mapping = BTreeMap::new();
    let mut used_headers: BTreeMap<String, String> = BTreeMap::new();
    for logical in LOGICAL_FIELDS {
        let resolved = if let Some(explicit) = requested.get(*logical) {
            let explicit = explicit.trim();
            if explicit.is_empty() {
                None
            } else {
                let Some(header) = headers
                    .iter()
                    .find(|header| header.trim().eq_ignore_ascii_case(explicit))
                else {
                    return Err(ApiError::bad_request(
                        "mapping_invalid",
                        format!("Mapping for `{logical}` names a column that is not in the file."),
                    )
                    .with_field(logical)
                    .with_details(serde_json::json!({ "column": explicit })));
                };
                Some(header.clone())
            }
        } else {
            let alias = aliases_for(logical);
            let mut matches = headers
                .iter()
                .filter(|header| alias.contains(&canonical_header(header).as_str()))
                .collect::<Vec<_>>();
            matches.sort();
            matches.dedup();
            if matches.len() > 1 {
                return Err(ApiError::bad_request(
                    "ambiguous_column",
                    format!("More than one column maps to `{logical}`; choose one explicitly."),
                )
                .with_field(logical));
            }
            matches.first().map(|header| (*header).clone())
        };
        if let Some(header) = resolved {
            let key = header.trim().to_lowercase();
            if let Some(previous) = used_headers.insert(key, logical.to_string()) {
                return Err(ApiError::bad_request(
                    "mapping_duplicate",
                    format!("Column `{header}` is mapped to both `{previous}` and `{logical}`."),
                )
                .with_field(logical));
            }
            mapping.insert(logical.to_string(), header);
        }
    }
    let missing = REQUIRED_FIELDS
        .iter()
        .filter(|field| !mapping.contains_key(**field))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(ApiError::bad_request(
            "mapping_missing",
            format!("Required columns are missing: {}.", missing.join(", ")),
        )
        .with_field(missing[0])
        .with_details(serde_json::json!({ "missing": missing })));
    }
    Ok(mapping)
}

fn column_index(
    headers: &[String],
    mapping: &BTreeMap<String, String>,
    logical: &str,
) -> Option<usize> {
    let header = mapping.get(logical)?;
    headers
        .iter()
        .position(|value| value.trim() == header.trim())
}

fn cell(record: &csv::StringRecord, index: Option<usize>) -> String {
    index
        .and_then(|position| record.get(position))
        .unwrap_or("")
        .trim()
        .to_string()
}

/// Parse and validate a preview. The returned preview is not persisted here.
pub fn preview_csv(
    bytes: &[u8],
    raw_filename: &str,
    currency_input: &str,
    requested_mapping: &BTreeMap<String, String>,
    preview_id: String,
) -> Result<ParsedCsv, ApiError> {
    reject_oversize(bytes.len())?;
    let text = std::str::from_utf8(bytes).map_err(|_| {
        ApiError::bad_request(
            "not_utf8",
            "The file is not valid UTF-8. Export it as UTF-8 CSV and try again.",
        )
        .with_field("file")
    })?;
    let text = text.trim_start_matches('\u{feff}');
    if text.trim().is_empty() {
        return Err(ApiError::bad_request(
            "empty_file",
            "The file is empty. Add a header row and at least one inventory row.",
        )
        .with_field("file"));
    }

    let batch_currency = crate::domain::parse_currency(currency_input).map_err(|error| {
        ApiError::bad_request(error.code.as_str(), error.message.as_str()).with_field(&error.field)
    })?;

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(text.as_bytes());
    let headers = reader
        .headers()
        .map_err(|error| {
            ApiError::bad_request(
                "csv_parse_error",
                format!("The header row could not be read: {error}"),
            )
            .with_field("file")
        })?
        .iter()
        .map(|value| value.trim().to_string())
        .collect::<Vec<_>>();
    if headers.is_empty() || headers.iter().all(|header| header.is_empty()) {
        return Err(ApiError::bad_request(
            "csv_headers_missing",
            "The first row must name the columns to import.",
        )
        .with_field("file"));
    }
    let mapping = resolve_mapping(&headers, requested_mapping)?;
    let indexes = LOGICAL_FIELDS
        .iter()
        .map(|logical| (*logical, column_index(&headers, &mapping, logical)))
        .collect::<BTreeMap<_, _>>();

    let filename = safe_filename(raw_filename);
    let reference = format!("csv:{filename}");
    let observed_at = now_rfc3339();
    let mut rows = Vec::new();
    let mut parse_failed = false;
    for (index, record) in reader.records().enumerate() {
        let record = match record {
            Ok(record) => record,
            Err(error) => {
                parse_failed = true;
                let row_number = error
                    .position()
                    .map(|position| position.line() as usize)
                    .unwrap_or(index + 2);
                rows.push(PreviewRow {
                    row: row_number,
                    status: RowStatus::Error,
                    values: RowValues::default(),
                    normalized: Default::default(),
                    specs: Vec::new(),
                    errors: vec![crate::domain::FieldError::new(
                        "file",
                        "csv_row_parse_error",
                        format!("This row could not be parsed: {error}"),
                    )
                    .suggest("Check quoting and separate every column with a comma.")],
                    duplicate_group_id: None,
                    provenance: BTreeMap::new(),
                    match_info: Default::default(),
                });
                continue;
            }
        };
        let row_number = record
            .position()
            .map(|position| position.line() as usize)
            .unwrap_or(index + 2);
        let values = RowValues {
            sku: cell(&record, indexes["sku"]),
            category: cell(&record, indexes["category"]),
            model: cell(&record, indexes["model"]),
            quantity: cell(&record, indexes["quantity"]),
            price: cell(&record, indexes["price"]),
            currency: cell(&record, indexes["currency"]),
            specs: cell(&record, indexes["specs"]),
            source: cell(&record, indexes["source"]),
            source_url: cell(&record, indexes["sourceUrl"]),
        };
        let mut evaluated = evaluate_row(&values, &batch_currency, &reference, &observed_at);
        if record.len() > headers.len() {
            evaluated.errors.push(
                crate::domain::FieldError::new(
                    "file",
                    "extra_columns",
                    format!(
                        "This row has {} fields but the header row has {}.",
                        record.len(),
                        headers.len()
                    ),
                )
                .suggest("Remove extra values or quote a value that contains a comma."),
            );
        }
        let status = if evaluated.errors.is_empty() {
            RowStatus::Valid
        } else {
            RowStatus::Error
        };
        rows.push(PreviewRow {
            row: row_number,
            status,
            values,
            normalized: evaluated.normalized,
            specs: evaluated.specs,
            errors: evaluated.errors,
            duplicate_group_id: None,
            provenance: evaluated.provenance,
            match_info: evaluated.match_info,
        });
    }
    if rows.is_empty() {
        return Err(ApiError::bad_request(
            "no_data_rows",
            "The file has a header row but no inventory rows.",
        )
        .with_field("file"));
    }
    if parse_failed && rows.iter().all(|row| row.status == RowStatus::Error) {
        return Err(ApiError::bad_request(
            "csv_parse_error",
            "No row could be parsed. Fix the CSV quoting and try again.",
        )
        .with_field("file"));
    }

    // Duplicate groups are built from every row that has a usable SKU, even if
    // another field is invalid; excluding the invalid row is then explicit.
    let mut group_by_sku: BTreeMap<String, usize> = BTreeMap::new();
    let mut groups: Vec<DuplicateGroup> = Vec::new();
    for row in &rows {
        let Some(sku) = row.normalized.sku.clone() else {
            continue;
        };
        let key = normalize_sku(&sku);
        if key.is_empty() {
            continue;
        }
        if let Some(group_index) = group_by_sku.get(&key) {
            groups[*group_index].rows.push(row.row);
        } else {
            let group_index = groups.len();
            group_by_sku.insert(key, group_index);
            groups.push(DuplicateGroup {
                group_id: format!("dup-{}", group_index + 1),
                sku,
                rows: vec![row.row],
            });
        }
    }
    groups.retain(|group| group.rows.len() > 1);
    for row in &mut rows {
        let Some(sku) = row.normalized.sku.as_deref() else {
            continue;
        };
        if let Some(group) = groups
            .iter()
            .find(|group| normalize_sku(group.sku.as_str()) == normalize_sku(sku))
        {
            row.duplicate_group_id = Some(group.group_id.clone());
            if row.status == RowStatus::Valid {
                row.status = RowStatus::Duplicate;
            }
        }
    }

    let summary = PreviewSummary {
        total: rows.len(),
        valid: rows
            .iter()
            .filter(|row| row.status == RowStatus::Valid)
            .count(),
        errors: rows
            .iter()
            .filter(|row| row.status == RowStatus::Error)
            .count(),
        duplicate_rows: rows
            .iter()
            .filter(|row| row.status == RowStatus::Duplicate)
            .count(),
    };
    let created_at = observed_at.clone();
    let expires = expires_at(&created_at);
    Ok(ParsedCsv {
        preview: Preview {
            preview_id,
            created_at,
            expires_at: expires,
            currency: batch_currency,
            filename,
            columns: headers,
            mapping,
            rows,
            duplicate_groups: groups,
            summary,
        },
        sha256: csv_checksum(bytes),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map() -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    #[test]
    fn valid_csv_maps_and_normalizes_rows() {
        let parsed = preview_csv(
            SAMPLE_CSV.as_bytes(),
            "sample.csv",
            "usd",
            &map(),
            "preview-1".to_string(),
        )
        .unwrap();
        assert_eq!(parsed.preview.currency, "USD");
        assert_eq!(parsed.preview.summary.total, 10);
        assert_eq!(parsed.preview.summary.errors, 0);
        assert_eq!(
            parsed.preview.rows[0].normalized.category.as_deref(),
            Some("cpu")
        );
        assert_eq!(parsed.preview.rows[0].normalized.price_minor, Some(44900));
        assert_eq!(parsed.preview.rows[0].specs.len(), 3);
        assert_eq!(parsed.preview.rows[0].provenance["sku"].source, "user");
        assert_eq!(
            parsed.preview.rows[0].provenance["model"].source,
            "catalog:manufacturer@2026-10-31"
        );
        assert_eq!(
            parsed.preview.rows[0].match_info.confidence,
            "catalog-declared"
        );
    }

    #[test]
    fn missing_required_columns_are_reported_without_parsing() {
        let error = preview_csv(
            b"sku,model,quantity\nA,X,1\n",
            "bad.csv",
            "USD",
            &map(),
            "preview-1".to_string(),
        )
        .unwrap_err();
        assert_eq!(error.code, "mapping_missing");
        assert!(error.message.contains("category"));
    }

    #[test]
    fn explicit_mapping_selects_a_column() {
        let mut requested = BTreeMap::new();
        requested.insert("price".to_string(), "Sale Price".to_string());
        let parsed = preview_csv(
            b"SKU,Type,Model,Qty,Sale Price\nA,cpu,X,1,10\n",
            "mapped.csv",
            "USD",
            &requested,
            "preview-1".to_string(),
        )
        .unwrap();
        assert_eq!(parsed.preview.mapping["price"], "Sale Price");
        assert_eq!(parsed.preview.rows[0].normalized.price_minor, Some(1000));
    }

    #[test]
    fn invalid_utf8_is_rejected() {
        let error = preview_csv(
            &[0xff, 0xfe, 0x00],
            "bad.csv",
            "USD",
            &map(),
            "preview-1".to_string(),
        )
        .unwrap_err();
        assert_eq!(error.code, "not_utf8");
        assert_eq!(error.status, axum::http::StatusCode::BAD_REQUEST);
    }

    #[test]
    fn duplicate_groups_and_field_errors_are_explicit() {
        let csv = "sku,category,model,quantity,price,currency,specs\n\
A,cpu,One,1,10,USD,socket=AM5\n\
A,cpu,One,2,10,USD,socket=AM5\n\
B,cpu,Two,abc,10,USD,\n\
C,cpu,Three,1,,USD,\n\
D,cpu,Four,1,10,EUR,\n";
        let parsed = preview_csv(csv.as_bytes(), "mixed.csv", "USD", &map(), "p".into()).unwrap();
        assert_eq!(parsed.preview.summary.errors, 3);
        assert_eq!(parsed.preview.summary.duplicate_rows, 2);
        assert_eq!(parsed.preview.duplicate_groups.len(), 1);
        assert_eq!(parsed.preview.duplicate_groups[0].rows.len(), 2);
        let codes = parsed
            .preview
            .rows
            .iter()
            .flat_map(|row| row.errors.iter().map(|error| error.code.as_str()))
            .collect::<Vec<_>>();
        assert!(codes.contains(&"quantity_format"));
        assert!(codes.contains(&"required"));
        assert!(codes.contains(&"currency_mismatch"));
    }

    #[test]
    fn duplicate_header_aliases_require_explicit_mapping() {
        let error = preview_csv(
            b"sku,category,model,quantity,price,unit price\nA,cpu,X,1,10,11\n",
            "ambiguous.csv",
            "USD",
            &map(),
            "p".into(),
        )
        .unwrap_err();
        assert_eq!(error.code, "ambiguous_column");
    }

    #[test]
    fn sample_csv_is_itself_valid() {
        let parsed = preview_csv(
            SAMPLE_CSV.as_bytes(),
            "sample.csv",
            "USD",
            &map(),
            "p".into(),
        )
        .unwrap();
        assert!(parsed.preview.rows.iter().all(|row| row.errors.is_empty()));
    }
}
