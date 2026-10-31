use super::*;

fn user() -> Provenance {
    Provenance::user("csv:test.csv", "2026-10-31T00:00:00Z")
}

#[test]
fn categories_normalize_aliases_and_reject_unknown() {
    assert_eq!(normalize_category(" Processor "), Some("cpu"));
    assert_eq!(normalize_category("mainboard"), Some("motherboard"));
    assert_eq!(normalize_category("Power Supply"), Some("psu"));
    assert_eq!(normalize_category("fan"), None);
}

#[test]
fn model_normalization_is_lexical_only() {
    assert_eq!(normalize_model("  ASUS   ROG  "), "ASUS ROG");
}

#[test]
fn money_parses_minor_units_and_rejects_precision() {
    assert_eq!(parse_money_minor("1299").unwrap(), 129900);
    assert_eq!(parse_money_minor("1299.5").unwrap(), 129950);
    assert_eq!(parse_money_minor("0.05").unwrap(), 5);
    assert_eq!(
        parse_money_minor("1.234").unwrap_err().code,
        "price_precision"
    );
    assert_eq!(parse_money_minor("-1").unwrap_err().code, "price_negative");
    assert_eq!(parse_money_minor("").unwrap_err().code, "required");
}

#[test]
fn quantity_rejects_fraction_and_negative() {
    assert_eq!(parse_quantity("3").unwrap(), 3);
    assert_eq!(parse_quantity("2.5").unwrap_err().code, "quantity_format");
    assert_eq!(parse_quantity("-2").unwrap_err().code, "quantity_negative");
}

#[test]
fn currency_requires_iso_shape() {
    assert_eq!(parse_currency("usd").unwrap(), "USD");
    assert_eq!(parse_currency("US").unwrap_err().code, "currency_format");
}

#[test]
fn specs_are_typed_and_unknown_keys_stay_unknown() {
    let specs = parse_specs("socket=AM5;tdp=105W;color=black", &user()).unwrap();
    assert_eq!(specs.len(), 3);
    assert_eq!(specs[0].key, "socket");
    assert_eq!(specs[0].status, SpecStatus::Confirmed);
    assert_eq!(specs[1].number, Some(105.0));
    assert_eq!(specs[1].unit.as_deref(), Some("W"));
    assert_eq!(specs[2].status, SpecStatus::Unknown);
    assert_eq!(specs[2].text.as_deref(), Some("black"));
}

#[test]
fn specs_reject_missing_unit_and_duplicates() {
    assert_eq!(
        parse_specs("tdp=105", &user()).unwrap_err().code,
        "spec_unit"
    );
    assert_eq!(
        parse_specs("tdp=105W;tdp=95W", &user()).unwrap_err().code,
        "spec_duplicate"
    );
    assert_eq!(
        parse_specs("socketAM5", &user()).unwrap_err().code,
        "spec_format"
    );
}

#[test]
fn evaluated_row_keeps_provenance_and_reports_fields() {
    let values = RowValues {
        sku: " CPU-1 ".into(),
        category: "cpu".into(),
        model: " Ryzen  7 ".into(),
        quantity: "2".into(),
        price: "299.99".into(),
        currency: "usd".into(),
        specs: "socket=AM5".into(),
        ..RowValues::default()
    };
    let row = evaluate_row(&values, "USD", "csv:test.csv", "2026-10-31T00:00:00Z");
    assert!(row.errors.is_empty());
    assert_eq!(row.normalized.sku.as_deref(), Some("CPU-1"));
    assert_eq!(row.normalized.model_normalized.as_deref(), Some("Ryzen 7"));
    assert_eq!(row.normalized.price_minor, Some(29999));
    assert_eq!(row.provenance["sku"].source, "user");
    assert_eq!(
        row.provenance["modelNormalized"].source,
        "rigcraft-normalizer"
    );
    assert_eq!(row.match_info.confidence, "unverified");
}

#[test]
fn catalog_source_routes_facts_but_not_stock() {
    let values = RowValues {
        sku: "A".into(),
        category: "cpu".into(),
        model: "Kit".into(),
        quantity: "2".into(),
        price: "10".into(),
        currency: "USD".into(),
        specs: "socket=AM5;tdp=120W".into(),
        source: "catalog:buildcores@abc1234".into(),
        source_url: "https://example.test/cpu".into(),
    };
    let row = evaluate_row(&values, "USD", "csv:test.csv", "2026-10-31T00:00:00Z");
    assert!(row.errors.is_empty());
    for field in ["sku", "quantity", "price", "currency"] {
        assert_eq!(row.provenance[field].source, "user", "{field}");
    }
    for field in ["model", "specs"] {
        assert_eq!(row.provenance[field].source, "catalog:buildcores@abc1234");
        assert_eq!(row.provenance[field].reference, "https://example.test/cpu");
    }
    assert_eq!(row.specs[0].provenance.source, "catalog:buildcores@abc1234");
    assert_eq!(row.normalized.category.as_deref(), Some("cpu"));
    assert_eq!(row.provenance["category"].source, "rigcraft-normalizer");
    assert_eq!(row.match_info.confidence, "catalog-declared");
    assert!(row.match_info.basis.contains("catalog:buildcores@abc1234"));
}

#[test]
fn catalog_source_without_url_falls_back_to_the_csv_reference() {
    let values = RowValues {
        sku: "A".into(),
        category: "ram".into(),
        model: "Kit".into(),
        quantity: "1".into(),
        price: "10".into(),
        currency: "USD".into(),
        specs: "capacity=32GB".into(),
        source: "catalog:open-db".into(),
        source_url: String::new(),
    };
    let row = evaluate_row(&values, "USD", "csv:stock.csv", "2026-10-31T00:00:00Z");
    assert!(row.errors.is_empty());
    assert_eq!(row.provenance["specs"].reference, "csv:stock.csv");
}

#[test]
fn invalid_catalog_claims_are_field_errors() {
    for source in ["shop", "catalog:", "catalog:has space", "catalog:a/b?c"] {
        let values = RowValues {
            sku: "A".into(),
            category: "cpu".into(),
            model: "Kit".into(),
            quantity: "1".into(),
            price: "10".into(),
            currency: "USD".into(),
            specs: String::new(),
            source: source.into(),
            source_url: String::new(),
        };
        let row = evaluate_row(&values, "USD", "csv:x.csv", "2026-10-31T00:00:00Z");
        assert_eq!(row.errors[0].code, "source_format", "{source}");
        assert_eq!(row.errors[0].field, "source");
        assert_eq!(row.provenance["model"].source, "user");
    }

    let values = RowValues {
        sku: "A".into(),
        category: "cpu".into(),
        model: "Kit".into(),
        quantity: "1".into(),
        price: "10".into(),
        currency: "USD".into(),
        specs: String::new(),
        source: "catalog:open-db".into(),
        source_url: "ftp://example.test".into(),
    };
    let row = evaluate_row(&values, "USD", "csv:x.csv", "2026-10-31T00:00:00Z");
    assert_eq!(row.errors[0].code, "source_url_format");
    assert_eq!(row.errors[0].field, "sourceUrl");
}

#[test]
fn user_source_is_operator_provenance() {
    let values = RowValues {
        sku: "A".into(),
        category: "cpu".into(),
        model: "Kit".into(),
        quantity: "1".into(),
        price: "10".into(),
        currency: "USD".into(),
        specs: "socket=AM5".into(),
        source: "user".into(),
        source_url: "https://example.test".into(),
    };
    let row = evaluate_row(&values, "USD", "csv:x.csv", "2026-10-31T00:00:00Z");
    assert!(row.errors.is_empty());
    assert_eq!(row.provenance["model"].source, "user");
    assert_eq!(row.provenance["model"].reference, "csv:x.csv");
    assert_eq!(row.match_info.confidence, "unverified");
}

#[test]
fn mixed_currency_is_a_field_error() {
    let values = RowValues {
        sku: "A".into(),
        category: "ram".into(),
        model: "Kit".into(),
        quantity: "1".into(),
        price: "10".into(),
        currency: "EUR".into(),
        specs: String::new(),
        ..RowValues::default()
    };
    let row = evaluate_row(&values, "USD", "csv:test.csv", "2026-10-31T00:00:00Z");
    assert_eq!(row.errors[0].code, "currency_mismatch");
    assert_eq!(row.errors[0].field, "currency");
}

#[test]
fn preview_expiry_is_calculated_from_creation_time() {
    let created = "2026-10-31T00:00:00Z";
    let now = OffsetDateTime::parse("2026-11-01T01:00:00Z", &Rfc3339).unwrap();
    assert!(preview_is_expired(created, now));
    assert!(!preview_is_expired(created, now - Duration::hours(2)));
    assert!(preview_is_expired("not-a-timestamp", now));
}

fn total_record(price_minor: i64, quantity: i64) -> Record {
    Record {
        id: "record".to_string(),
        lot: 1,
        sku: "SKU".to_string(),
        category: "cpu".to_string(),
        model_raw: "Part".to_string(),
        model_normalized: "Part".to_string(),
        quantity,
        price_minor,
        currency: "USD".to_string(),
        specs: Vec::new(),
        provenance: BTreeMap::new(),
        match_info: MatchInfo::default(),
    }
}

#[test]
fn totals_use_checked_arithmetic_and_reject_overflow() {
    let totals = Snapshot::totals_for(&[total_record(100, 3)]).unwrap();
    assert_eq!(totals.unit_count, 3);
    assert_eq!(totals.value_minor, 300);

    let line_overflow = Snapshot::totals_for(&[total_record(i64::MAX, 2)]);
    assert_eq!(line_overflow, Err(TotalsOverflow));

    let sum_overflow = Snapshot::totals_for(&[
        total_record(i64::MAX / 2 + 1, 1),
        total_record(i64::MAX / 2 + 1, 1),
    ]);
    assert_eq!(sum_overflow, Err(TotalsOverflow));

    let unit_overflow = Snapshot::totals_for(&[total_record(0, i64::MAX), total_record(0, 1)]);
    assert_eq!(unit_overflow, Err(TotalsOverflow));
}
