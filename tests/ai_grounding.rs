//! Grounding tests for the BYOK interpretation layer.
//!
//! These use only the public interpretation surface, so the unit test of record
//! lives beside the other integration tests and the source file stays small.

use rigcraft::ai::{field_changes, interpret, revised_request, CurrentRequest, FieldSource};
use rigcraft::domain::{MatchInfo, Provenance, Record, Snapshot, SnapshotSource};
use serde_json::Value;
use std::collections::BTreeMap;

fn record(sku: &str, category: &str) -> Record {
    let user = Provenance::user("csv:ai.csv", "2026-10-31T00:00:00Z");
    Record {
        id: format!("id-{sku}"),
        lot: 1,
        sku: sku.to_string(),
        category: category.to_string(),
        model_raw: format!("Fixture {sku}"),
        model_normalized: format!("Fixture {sku}"),
        quantity: 1,
        price_minor: 10000,
        currency: "USD".to_string(),
        specs: Vec::new(),
        provenance: BTreeMap::from([("model".to_string(), user)]),
        match_info: MatchInfo::default(),
    }
}

fn snapshot() -> Snapshot {
    let records = vec![
        record("FIX-CPU", "cpu"),
        record("FIX-GPU", "gpu"),
        record("FIX-RAM", "ram"),
    ];
    Snapshot {
        id: "snapshot-1".to_string(),
        created_at: "2026-10-31T00:00:00Z".to_string(),
        revision: 1,
        parent_snapshot_id: None,
        currency: "USD".to_string(),
        source: SnapshotSource {
            filename: "ai.csv".to_string(),
            sha256: "abc".to_string(),
        },
        note: None,
        totals: Snapshot::totals_for(&records),
        records,
    }
}

fn reply(value: Value) -> String {
    value.to_string()
}

fn confident_reply() -> Value {
    serde_json::json!({
        "budgetMinor": { "value": 120000, "excerpt": "$1200" },
        "currency": { "value": "USD", "excerpt": "USD" },
        "useCase": { "value": "gaming", "excerpt": "gaming" },
        "inventoryOnly": { "value": true, "excerpt": "only my stock" },
        "lockedSkus": { "value": ["FIX-CPU"], "excerpt": "keep FIX-CPU" },
        "excludedSkus": { "value": [], "excerpt": "" }
    })
}

const SENTENCE: &str = "Build a $1200 USD gaming PC from only my stock and keep FIX-CPU.";

#[test]
fn a_grounded_reply_becomes_a_proposal_and_a_request() {
    let snapshot = snapshot();
    let interpretation = interpret(
        &reply(confident_reply()),
        SENTENCE,
        Some(&snapshot),
        "snapshot-1",
        &[],
    )
    .unwrap();
    assert_eq!(interpretation.proposal.budget_minor.value(), Some(&120000));
    assert_eq!(
        interpretation.proposal.budget_minor.source,
        FieldSource::Sentence
    );
    assert_eq!(
        interpretation.proposal.locked_skus.value(),
        Some(&vec!["FIX-CPU".to_string()])
    );
    assert!(interpretation.ignored_fields.is_empty());
    let request = interpretation.request.unwrap();
    assert_eq!(request.budget_minor, Some(120000));
    assert_eq!(request.use_case.as_deref(), Some("gaming"));
    assert_eq!(request.locked_skus, vec!["FIX-CPU".to_string()]);
    assert_eq!(request.inventory_only, Some(true));
}

#[test]
fn a_value_that_is_not_in_the_sentence_is_unresolved() {
    let snapshot = snapshot();
    let mut value = confident_reply();
    value["budgetMinor"] = serde_json::json!({ "value": 900000, "excerpt": "$9000" });
    let interpretation =
        interpret(&reply(value), SENTENCE, Some(&snapshot), "snapshot-1", &[]).unwrap();
    assert_eq!(
        interpretation.proposal.budget_minor.source,
        FieldSource::Unresolved
    );
    assert!(interpretation
        .ambiguities
        .iter()
        .any(|entry| entry.field == "budgetMinor" && entry.code == "ungrounded_value"));
    assert!(interpretation.request.is_none());
}

#[test]
fn an_invented_part_is_dropped_and_reported() {
    let snapshot = snapshot();
    let mut value = confident_reply();
    value["lockedSkus"] =
        serde_json::json!({ "value": ["FIX-CPU", "FIX-INVENTED"], "excerpt": "keep FIX-CPU" });
    let interpretation =
        interpret(&reply(value), SENTENCE, Some(&snapshot), "snapshot-1", &[]).unwrap();
    assert_eq!(
        interpretation.proposal.locked_skus.value(),
        Some(&vec!["FIX-CPU".to_string()])
    );
    assert!(interpretation
        .ambiguities
        .iter()
        .any(|entry| entry.code == "unknown_sku" && entry.message.contains("FIX-INVENTED")));
}

#[test]
fn a_part_can_only_come_from_the_sentence_or_an_included_part_list() {
    let snapshot = snapshot();
    let mut value = confident_reply();
    value["lockedSkus"] = serde_json::json!({ "value": ["FIX-GPU"], "excerpt": "keep FIX-CPU" });
    let without_list = interpret(
        &reply(value.clone()),
        SENTENCE,
        Some(&snapshot),
        "snapshot-1",
        &[],
    )
    .unwrap();
    assert!(without_list
        .ambiguities
        .iter()
        .any(|entry| entry.code == "ungrounded_value" && entry.field == "lockedSkus"));
    assert!(without_list.request.is_none());

    let with_list = interpret(
        &reply(value),
        SENTENCE,
        Some(&snapshot),
        "snapshot-1",
        &["FIX-CPU".to_string(), "FIX-GPU".to_string()],
    )
    .unwrap();
    assert_eq!(with_list.proposal.locked_skus.source, FieldSource::Snapshot);
    assert_eq!(
        with_list.proposal.locked_skus.value(),
        Some(&vec!["FIX-GPU".to_string()])
    );
}

#[test]
fn inventory_only_is_labelled_as_a_default_when_the_sentence_is_silent() {
    let snapshot = snapshot();
    let mut value = confident_reply();
    value.as_object_mut().unwrap().remove("inventoryOnly");
    let interpretation =
        interpret(&reply(value), SENTENCE, Some(&snapshot), "snapshot-1", &[]).unwrap();
    assert_eq!(interpretation.proposal.inventory_only.value(), Some(&true));
    assert_eq!(
        interpretation.proposal.inventory_only.source,
        FieldSource::Default
    );
}

#[test]
fn the_snapshot_supplies_a_currency_the_sentence_omits() {
    let snapshot = snapshot();
    let mut value = confident_reply();
    value.as_object_mut().unwrap().remove("currency");
    let interpretation =
        interpret(&reply(value), SENTENCE, Some(&snapshot), "snapshot-1", &[]).unwrap();
    assert_eq!(
        interpretation.proposal.currency.value().map(String::as_str),
        Some("USD")
    );
    assert_eq!(
        interpretation.proposal.currency.source,
        FieldSource::Snapshot
    );
}

#[test]
fn a_missing_use_case_and_budget_are_ambiguities_not_guesses() {
    let snapshot = snapshot();
    let interpretation = interpret(
        &reply(serde_json::json!({ "lockedSkus": { "value": [], "excerpt": "" } })),
        "make me a computer",
        Some(&snapshot),
        "snapshot-1",
        &[],
    )
    .unwrap();
    let fields: Vec<&str> = interpretation
        .ambiguities
        .iter()
        .map(|entry| entry.field.as_str())
        .collect();
    assert!(fields.contains(&"budgetMinor"));
    assert!(fields.contains(&"useCase"));
    assert!(interpretation.request.is_none());
    assert_eq!(interpretation.proposal.budget_minor.value(), None);
}

#[test]
fn a_catalog_request_is_reported_and_never_becomes_a_request() {
    let snapshot = snapshot();
    let mut value = confident_reply();
    value["inventoryOnly"] = serde_json::json!({ "value": false, "excerpt": "use the catalog" });
    let sentence =
        "Build a $1200 USD gaming PC from only my stock and keep FIX-CPU, use the catalog";
    let interpretation =
        interpret(&reply(value), sentence, Some(&snapshot), "snapshot-1", &[]).unwrap();
    assert_eq!(interpretation.proposal.inventory_only.value(), Some(&false));
    assert!(interpretation
        .ambiguities
        .iter()
        .any(|entry| entry.code == "catalog_unavailable"));
    assert!(interpretation.request.is_none());
}

#[test]
fn field_values_outside_the_contract_are_discarded() {
    let snapshot = snapshot();
    let mut value = confident_reply();
    value["performanceScore"] = serde_json::json!({ "value": 99, "excerpt": "fast" });
    value["compatibility"] = serde_json::json!("compatible");
    let interpretation =
        interpret(&reply(value), SENTENCE, Some(&snapshot), "snapshot-1", &[]).unwrap();
    assert!(interpretation
        .ignored_fields
        .contains(&"performanceScore".to_string()));
    assert!(interpretation
        .ignored_fields
        .contains(&"compatibility".to_string()));
    let serialized = serde_json::to_string(&interpretation.proposal).unwrap();
    assert!(!serialized.contains("performanceScore"));
    assert!(!serialized.contains("compatible"));
}

#[test]
fn a_revision_only_names_the_fields_it_changes() {
    let snapshot = snapshot();
    let current = CurrentRequest {
        budget_minor: Some(120000),
        currency: Some("USD".to_string()),
        use_case: Some("gaming".to_string()),
        locked_skus: vec!["FIX-CPU".to_string()],
        excluded_skus: Vec::new(),
    };
    let interpretation = interpret(
        &reply(serde_json::json!({
            "budgetMinor": { "value": 150000, "excerpt": "1500" }
        })),
        "raise the budget to 1500",
        Some(&snapshot),
        "snapshot-1",
        &[],
    )
    .unwrap();
    let changes = field_changes(&interpretation.proposal, &current);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].field, "budgetMinor");
    assert_eq!(changes[0].to, serde_json::json!(150000));
    let applied = revised_request(&interpretation.proposal, &current, "snapshot-1").unwrap();
    assert_eq!(applied.budget_minor, Some(150000));
    assert_eq!(applied.use_case.as_deref(), Some("gaming"));
    assert_eq!(applied.locked_skus, vec!["FIX-CPU".to_string()]);
}

#[test]
fn a_part_that_is_both_kept_and_excluded_is_resolved_and_reported() {
    let snapshot = snapshot();
    let mut value = confident_reply();
    value["excludedSkus"] = serde_json::json!({ "value": ["FIX-CPU"], "excerpt": "keep FIX-CPU" });
    let interpretation =
        interpret(&reply(value), SENTENCE, Some(&snapshot), "snapshot-1", &[]).unwrap();
    assert_eq!(
        interpretation.proposal.excluded_skus.value(),
        Some(&Vec::<String>::new())
    );
    assert!(interpretation
        .ambiguities
        .iter()
        .any(|entry| entry.code == "locked_excluded_conflict"));
}
