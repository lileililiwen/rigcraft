//! Bounded deterministic search over one snapshot, plus scoring and ranking.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use crate::compatibility::{
    check_build, CheckReport, CheckRequest, CompletenessStatus, SelectionItem, Verdict,
};
use crate::domain::{Record, Snapshot, SpecStatus};
use crate::error::ApiError;

use super::candidate::make_candidate;
use super::explain::{
    cheapest_in, compatibility_solution, minimum_complete_cost, required_cheapest_blockers,
    search_limit_solution,
};
use super::score::score_build;
use super::{
    format_money, Blocker, Exclusion, NoSolution, ScoreBreakdown, SearchOutcome, SearchStats,
    Settings, MAX_CANDIDATES, SEARCH_WORK_LIMIT,
};

pub(crate) const CATEGORY_ORDER: [&str; 8] = [
    "cpu",
    "motherboard",
    "ram",
    "gpu",
    "psu",
    "case",
    "cooler",
    "storage",
];
const REQUIRED: [&str; 6] = ["cpu", "motherboard", "ram", "psu", "case", "storage"];
/// How many highest-scoring valid builds are retained for the alternative fill.
const TOP_POOL: usize = 32;
const METHOD: &str = "Deterministic depth-first enumeration in category order \
(cpu, motherboard, ram, gpu, psu, case, cooler, storage). Over-budget partial builds \
are pruned; every complete combination is checked with the compatibility rule set before \
it can be a candidate; expansion stops at the stated node limit.";

pub(crate) struct Piece {
    pub record: Record,
    pub quantity: i64,
    pub owned: bool,
    pub line_total_minor: i64,
}

pub(crate) struct Pool {
    pub category: &'static str,
    pub required: bool,
    pub options: Vec<Piece>,
}

pub(crate) struct SearchInput {
    pub pools: Vec<Pool>,
    pub exclusions: Vec<Exclusion>,
}

fn is_required(category: &str) -> bool {
    REQUIRED.contains(&category)
}

fn confirmed_text<'a>(record: &'a Record, key: &str) -> Option<&'a str> {
    record
        .specs
        .iter()
        .find(|spec| spec.key == key && spec.status == SpecStatus::Confirmed)
        .and_then(|spec| spec.text.as_deref())
}

pub(crate) fn confirmed_number(record: &Record, key: &str) -> Option<f64> {
    record
        .specs
        .iter()
        .find(|spec| spec.key == key && spec.status == SpecStatus::Confirmed)
        .and_then(|spec| {
            spec.number.or_else(|| {
                spec.text
                    .as_deref()
                    .and_then(|value| value.trim().parse::<f64>().ok())
            })
        })
}

pub(crate) fn fact_is_yes(record: &Record, key: &str) -> bool {
    matches!(
        confirmed_text(record, key)
            .map(|value| value.trim().to_lowercase())
            .as_deref(),
        Some("yes" | "true" | "1" | "confirmed")
    )
}

/// Merge lots of the same SKU the way the checker does, and report a SKU whose
/// lots disagree instead of silently picking one.
fn unique_records(snapshot: &Snapshot) -> (Vec<Record>, Vec<Exclusion>) {
    let mut grouped: BTreeMap<&str, Vec<&Record>> = BTreeMap::new();
    for record in &snapshot.records {
        grouped.entry(record.sku.as_str()).or_default().push(record);
    }
    let mut records = Vec::with_capacity(grouped.len());
    let mut exclusions = Vec::new();
    for (sku, lots) in grouped {
        let first = lots[0];
        let conflicting = lots.iter().any(|record| {
            record.category != first.category
                || record.model_normalized != first.model_normalized
                || record.specs != first.specs
        });
        if conflicting {
            exclusions.push(Exclusion {
                sku: sku.to_string(),
                category: first.category.clone(),
                model: first.model_normalized.clone(),
                reason: "ambiguous_sku".to_string(),
                detail: format!(
                    "SKU {sku} has {} lots with conflicting facts; correct or merge it before recommending.",
                    lots.len()
                ),
            });
            continue;
        }
        let mut merged = first.clone();
        merged.quantity = lots.iter().map(|record| record.quantity).sum();
        records.push(merged);
    }
    (records, exclusions)
}

fn empty_category_detail(snapshot: &Snapshot, settings: &Settings, category: &str) -> String {
    let records: Vec<&Record> = snapshot
        .records
        .iter()
        .filter(|record| record.category == category)
        .collect();
    let base = if records.is_empty() {
        format!("No {category} is recorded in this snapshot.")
    } else if records.iter().all(|record| record.quantity <= 0) {
        format!("Every {category} in this snapshot has zero available quantity.")
    } else if records
        .iter()
        .all(|record| settings.excluded_skus.contains(&record.sku))
    {
        format!("Every {category} in this snapshot is excluded by this request.")
    } else if records
        .iter()
        .all(|record| record.price_minor > settings.budget_minor)
    {
        format!("Every {category} costs more than the whole budget.")
    } else {
        format!(
            "No usable {category} remains after stock, price, currency, duplicate and exclusion checks."
        )
    };
    if category == "gpu" && settings.use_case == "gaming" {
        format!("{base} Gaming requires a discrete graphics card.")
    } else {
        base
    }
}

/// Build the fixed category pools after every hard filter. The result is fully
/// determined by the snapshot and the validated request.
pub(crate) fn build_input(
    snapshot: &Snapshot,
    settings: &Settings,
) -> Result<SearchInput, ApiError> {
    let (records, mut exclusions) = unique_records(snapshot);
    let mut pools = Vec::with_capacity(CATEGORY_ORDER.len());
    let locked_by_category: BTreeMap<String, &Record> = settings
        .locked_skus
        .iter()
        .filter_map(|sku| {
            records
                .iter()
                .find(|record| &record.sku == sku)
                .map(|record| (record.category.clone(), record))
        })
        .collect();

    for category in CATEGORY_ORDER {
        // Gaming is only served by a discrete graphics card; office and creation
        // keep the optional-graphics behaviour the compatibility engine models.
        let required =
            is_required(category) || (category == "gpu" && settings.use_case == "gaming");
        if let Some(locked) = locked_by_category.get(category) {
            if locked.quantity <= 0 {
                return Err(ApiError::unprocessable(
                    "locked_sku_out_of_stock",
                    format!(
                        "Locked SKU {} has no available stock, so it cannot be checked as part of a build.",
                        locked.sku
                    ),
                )
                .with_field("lockedSkus")
                .with_details(serde_json::json!({ "sku": locked.sku })));
            }
            pools.push(Pool {
                category,
                required,
                options: vec![Piece {
                    record: (*locked).clone(),
                    quantity: 1,
                    owned: true,
                    line_total_minor: 0,
                }],
            });
            for record in records
                .iter()
                .filter(|record| record.category == category && record.sku != locked.sku)
            {
                exclusions.push(Exclusion {
                    sku: record.sku.clone(),
                    category: record.category.clone(),
                    model: record.model_normalized.clone(),
                    reason: "locked_category".to_string(),
                    detail: format!(
                        "A locked {category} ({}) fixes this slot, so {sku} is not considered.",
                        locked.sku,
                        sku = record.sku
                    ),
                });
            }
            continue;
        }

        let mut options = Vec::new();
        let mut seen = BTreeSet::new();
        for record in records.iter().filter(|record| record.category == category) {
            if !seen.insert(record.sku.clone()) {
                continue;
            }
            let exclusion = if settings.excluded_skus.contains(&record.sku) {
                Some((
                    "requested_excluded",
                    format!("{} was excluded by this request.", record.sku),
                ))
            } else if record.quantity <= 0 {
                Some((
                    "out_of_stock",
                    format!("{} has no available quantity.", record.sku),
                ))
            } else if record.currency != settings.currency {
                Some((
                    "currency_mismatch",
                    format!(
                        "{} is priced in {}, not {}.",
                        record.sku, record.currency, settings.currency
                    ),
                ))
            } else if record.price_minor > settings.budget_minor {
                Some((
                    "over_budget",
                    format!(
                        "{} alone costs {} of the {} budget.",
                        record.sku,
                        format_money(record.price_minor, &settings.currency),
                        format_money(settings.budget_minor, &settings.currency)
                    ),
                ))
            } else {
                None
            };
            if let Some((reason, detail)) = exclusion {
                exclusions.push(Exclusion {
                    sku: record.sku.clone(),
                    category: record.category.clone(),
                    model: record.model_normalized.clone(),
                    reason: reason.to_string(),
                    detail,
                });
                continue;
            }
            let quantities: Vec<i64> = if category == "ram" {
                (1..=record.quantity.min(2)).collect()
            } else {
                vec![1]
            };
            for quantity in quantities {
                options.push(Piece {
                    record: record.clone(),
                    quantity,
                    owned: false,
                    line_total_minor: record.price_minor.saturating_mul(quantity),
                });
            }
        }
        options.sort_by(|left, right| {
            left.line_total_minor
                .cmp(&right.line_total_minor)
                .then(left.quantity.cmp(&right.quantity))
                .then(left.record.sku.cmp(&right.record.sku))
        });
        pools.push(Pool {
            category,
            required,
            options,
        });
    }
    Ok(SearchInput { pools, exclusions })
}

struct Walker<'a> {
    snapshot: &'a Snapshot,
    budget: i64,
    use_case: String,
    suffix_min: Vec<i64>,
    chosen: Vec<&'a Piece>,
    nodes: u64,
    leaves: u64,
    valid: u64,
    limited: bool,
    top: Vec<Rc<Evaluated>>,
    cheapest: Option<Rc<Evaluated>>,
    best_performance: Option<Rc<Evaluated>>,
    rejections: BTreeMap<String, u64>,
}

#[derive(Clone)]
pub(crate) struct PartCopy {
    pub(crate) record: Record,
    pub(crate) quantity: i64,
    pub(crate) owned: bool,
    pub(crate) line_total_minor: i64,
}

#[derive(Clone)]
pub(crate) struct Evaluated {
    pub(crate) signature: String,
    pub(crate) material_signature: String,
    pub(crate) total: i64,
    pub(crate) scores: ScoreBreakdown,
    pub(crate) performance: Option<f64>,
    pub(crate) parts: Vec<PartCopy>,
    pub(crate) report: CheckReport,
}

fn signature(parts: &[&Piece]) -> String {
    let mut entries: Vec<String> = parts
        .iter()
        .map(|piece| {
            format!(
                "{}:{}:{}",
                piece.record.category, piece.record.sku, piece.quantity
            )
        })
        .collect();
    entries.sort();
    entries.join("|")
}

/// The parts that make two machines different choices. Storage is excluded on
/// purpose: completeness requires one drive, and a larger capacity is not a
/// different build under REQ-R02.
fn material_signature(parts: &[&Piece]) -> String {
    let mut entries: Vec<String> = parts
        .iter()
        .filter(|piece| piece.record.category != "storage")
        .map(|piece| {
            format!(
                "{}:{}:{}",
                piece.record.category, piece.record.sku, piece.quantity
            )
        })
        .collect();
    entries.sort();
    entries.join("|")
}

/// Within one material signature the cheapest variant is the representative:
/// the cost dimension falls as the total rises, so it is also the best-scoring
/// variant. Score and the full signature make the order total and stable.
fn better_representative(
    total: i64,
    score: f64,
    signature: &str,
    other_total: i64,
    other_score: f64,
    other_signature: &str,
) -> Ordering {
    total
        .cmp(&other_total)
        .then(other_score.total_cmp(&score))
        .then(signature.cmp(other_signature))
}

fn better_by_score(
    score: f64,
    total: i64,
    signature: &str,
    other_score: f64,
    other_total: i64,
    other_signature: &str,
) -> Ordering {
    other_score
        .total_cmp(&score)
        .then(total.cmp(&other_total))
        .then(signature.cmp(other_signature))
}

fn better_by_cost(
    score: f64,
    total: i64,
    signature: &str,
    other_score: f64,
    other_total: i64,
    other_signature: &str,
) -> Ordering {
    total
        .cmp(&other_total)
        .then(other_score.total_cmp(&score))
        .then(signature.cmp(other_signature))
}

#[allow(clippy::too_many_arguments)]
fn better_by_performance(
    performance: Option<f64>,
    score: f64,
    total: i64,
    signature: &str,
    other_performance: Option<f64>,
    other_score: f64,
    other_total: i64,
    other_signature: &str,
) -> Ordering {
    let left = performance.unwrap_or(f64::NEG_INFINITY);
    let right = other_performance.unwrap_or(f64::NEG_INFINITY);
    right
        .total_cmp(&left)
        .then(other_score.total_cmp(&score))
        .then(total.cmp(&other_total))
        .then(signature.cmp(other_signature))
}

impl<'a> Walker<'a> {
    fn new(snapshot: &'a Snapshot, settings: &Settings, pools: &[Pool]) -> Self {
        let mut suffix_min = vec![0i64; pools.len() + 1];
        for index in (0..pools.len()).rev() {
            let cheapest = pools[index]
                .options
                .iter()
                .map(|piece| piece.line_total_minor)
                .min()
                .unwrap_or(0);
            let own = if pools[index].required { cheapest } else { 0 };
            suffix_min[index] = suffix_min[index + 1].saturating_add(own);
        }
        Self {
            snapshot,
            budget: settings.budget_minor,
            use_case: settings.use_case.clone(),
            suffix_min,
            chosen: Vec::with_capacity(pools.len()),
            nodes: 0,
            leaves: 0,
            valid: 0,
            limited: false,
            top: Vec::new(),
            cheapest: None,
            best_performance: None,
            rejections: BTreeMap::new(),
        }
    }

    fn bump(&mut self) {
        self.nodes += 1;
        if self.nodes >= SEARCH_WORK_LIMIT {
            self.limited = true;
        }
    }

    fn walk(&mut self, pools: &'a [Pool], index: usize, cost: i64) {
        if self.limited {
            return;
        }
        if index == pools.len() {
            self.leaf(cost);
            return;
        }
        let pool = &pools[index];
        if !pool.required {
            self.bump();
            if self.limited {
                return;
            }
            self.walk(pools, index + 1, cost);
            if self.limited {
                return;
            }
        }
        for piece in &pool.options {
            let next = cost.saturating_add(piece.line_total_minor);
            if next.saturating_add(self.suffix_min[index + 1]) > self.budget {
                continue;
            }
            self.bump();
            if self.limited {
                return;
            }
            self.chosen.push(piece);
            self.walk(pools, index + 1, next);
            self.chosen.pop();
            if self.limited {
                return;
            }
        }
    }

    fn leaf(&mut self, cost: i64) {
        self.leaves += 1;
        let mut selection: Vec<SelectionItem> = self
            .chosen
            .iter()
            .map(|piece| SelectionItem {
                sku: piece.record.sku.clone(),
                quantity: piece.quantity,
            })
            .collect();
        selection.sort_by(|left, right| left.sku.cmp(&right.sku));
        let report = match check_build(
            self.snapshot,
            &CheckRequest {
                snapshot_id: self.snapshot.id.clone(),
                selection,
            },
        ) {
            Ok(report) => report,
            Err(_) => {
                *self
                    .rejections
                    .entry("check_error".to_string())
                    .or_insert(0) += 1;
                return;
            }
        };
        if report.overall_status == Verdict::Incompatible {
            for rule in report
                .rules
                .iter()
                .filter(|rule| rule.status == Verdict::Incompatible)
            {
                *self
                    .rejections
                    .entry(format!("incompatible:{}", rule.rule_id))
                    .or_insert(0) += 1;
            }
            return;
        }
        if report.completeness.status == CompletenessStatus::Incomplete {
            for missing in &report.completeness.missing {
                *self
                    .rejections
                    .entry(format!("incomplete:{}", missing.category))
                    .or_insert(0) += 1;
            }
            return;
        }
        self.valid += 1;
        let scores = score_build(&self.chosen, cost, self.budget, &self.use_case);
        let performance = scores
            .dimensions
            .iter()
            .find(|dimension| dimension.key == "performance")
            .and_then(|dimension| dimension.value);
        let signature = signature(&self.chosen);
        let material_signature = material_signature(&self.chosen);
        let keep_top = match self.top.last() {
            None => true,
            Some(worst) => {
                self.top.len() < TOP_POOL
                    || self
                        .top
                        .iter()
                        .any(|entry| entry.material_signature == material_signature)
                    || better_by_score(
                        scores.total,
                        cost,
                        &signature,
                        worst.scores.total,
                        worst.total,
                        &worst.signature,
                    ) == Ordering::Less
            }
        };
        let keep_cheapest = match &self.cheapest {
            None => true,
            Some(current) => {
                better_by_cost(
                    scores.total,
                    cost,
                    &signature,
                    current.scores.total,
                    current.total,
                    &current.signature,
                ) == Ordering::Less
            }
        };
        let keep_performance = performance.is_some()
            && match &self.best_performance {
                None => true,
                Some(current) => {
                    better_by_performance(
                        performance,
                        scores.total,
                        cost,
                        &signature,
                        current.performance,
                        current.scores.total,
                        current.total,
                        &current.signature,
                    ) == Ordering::Less
                }
            };
        if !keep_top && !keep_cheapest && !keep_performance {
            return;
        }
        let parts: Vec<PartCopy> = self
            .chosen
            .iter()
            .map(|piece| PartCopy {
                record: piece.record.clone(),
                quantity: piece.quantity,
                owned: piece.owned,
                line_total_minor: piece.line_total_minor,
            })
            .collect();
        let top_key = material_signature.clone();
        let evaluated = Rc::new(Evaluated {
            signature,
            material_signature,
            total: cost,
            scores,
            performance,
            parts,
            report,
        });
        if keep_performance {
            self.best_performance = Some(Rc::clone(&evaluated));
        }
        if keep_cheapest {
            self.cheapest = Some(Rc::clone(&evaluated));
        }
        if keep_top {
            if let Some(index) = self
                .top
                .iter()
                .position(|entry| entry.material_signature == top_key)
            {
                let current = &self.top[index];
                if better_representative(
                    cost,
                    evaluated.scores.total,
                    &evaluated.signature,
                    current.total,
                    current.scores.total,
                    &current.signature,
                ) == Ordering::Less
                {
                    self.top[index] = Rc::clone(&evaluated);
                }
            } else {
                self.top.push(Rc::clone(&evaluated));
            }
            self.top.sort_by(|left, right| {
                better_by_score(
                    left.scores.total,
                    left.total,
                    &left.signature,
                    right.scores.total,
                    right.total,
                    &right.signature,
                )
            });
            if self.top.len() > TOP_POOL {
                self.top.truncate(TOP_POOL);
            }
        }
    }
}

pub(crate) fn category_min(pools: &[Pool], category: &str) -> Option<i64> {
    pools
        .iter()
        .find(|pool| pool.category == category)
        .and_then(|pool| {
            pool.options
                .iter()
                .map(|piece| piece.line_total_minor)
                .min()
        })
}

/// Cheapest structurally complete build ignoring compatibility. Optional
/// graphics and cooling are priced per CPU from its sourced substitute facts.
pub(crate) fn search_builds(
    snapshot: &Snapshot,
    settings: &Settings,
    pools: &[Pool],
    exclusions: Vec<Exclusion>,
) -> SearchOutcome {
    let mut stats = SearchStats {
        limit: SEARCH_WORK_LIMIT,
        method: METHOD.to_string(),
        ..SearchStats::default()
    };
    let currency = settings.currency.as_str();
    let gpu_required = settings.use_case == "gaming";

    let missing_required: Vec<&str> = pools
        .iter()
        .filter(|pool| pool.required && pool.options.is_empty())
        .map(|pool| pool.category)
        .collect();
    if !missing_required.is_empty() {
        let gpu_missing_for_gaming = gpu_required && missing_required.contains(&"gpu");
        let blockers = missing_required
            .iter()
            .map(|category| Blocker {
                code: "category_unavailable".to_string(),
                category: Some((*category).to_string()),
                sku: None,
                detail: empty_category_detail(snapshot, settings, category),
            })
            .collect();
        return SearchOutcome {
            candidates: Vec::new(),
            no_solution: Some(NoSolution {
                binding: "stock".to_string(),
                headline: if gpu_missing_for_gaming {
                    "No gaming build can be assembled without a discrete graphics card."
                        .to_string()
                } else {
                    "No complete build can be assembled from this snapshot.".to_string()
                },
                explanation: if gpu_missing_for_gaming {
                    format!(
                        "A gaming build requires a discrete graphics card, and this snapshot cannot supply one. It also cannot supply: {}.",
                        missing_required.join(", ")
                    )
                } else {
                    format!(
                        "A complete build needs {}. This snapshot cannot supply: {}.",
                        REQUIRED.join(", "),
                        missing_required.join(", ")
                    )
                },
                blockers,
                adjustable: vec![
                    "Import or restock the missing categories, then run the recommendation again."
                        .to_string(),
                    "A correction snapshot keeps the previous revision, so an existing export stays traceable."
                        .to_string(),
                ],
                minimum_build_minor: None,
                budget_shortfall_minor: None,
            }),
            search: stats,
            exclusions,
        };
    }

    let cpu_pool = pools.iter().find(|pool| pool.category == "cpu");
    let gpu_available = category_min(pools, "gpu").is_some();
    let cooler_available = category_min(pools, "cooler").is_some();
    let igpu_evidence = cpu_pool.is_some_and(|pool| {
        pool.options
            .iter()
            .any(|piece| fact_is_yes(&piece.record, "igpu"))
    });
    let bundled_evidence = cpu_pool.is_some_and(|pool| {
        pool.options
            .iter()
            .any(|piece| fact_is_yes(&piece.record, "bundled_cooler"))
    });
    let mut gaps = Vec::new();
    if !igpu_evidence && !gpu_available {
        gaps.push((
            "gpu",
            "no graphics card is in stock and no CPU has sourced `igpu=yes` evidence",
        ));
    }
    if !bundled_evidence && !cooler_available {
        gaps.push((
            "cooler",
            "no cooler is in stock and no CPU has sourced `bundled_cooler=yes` evidence",
        ));
    }
    if !gaps.is_empty() {
        return SearchOutcome {
            candidates: Vec::new(),
            no_solution: Some(NoSolution {
                binding: "compatibility".to_string(),
                headline: "No complete build can be formed from this stock.".to_string(),
                explanation: "The compatibility contract requires graphics and cooling evidence; this snapshot has neither the part nor the sourced substitute evidence.".to_string(),
                blockers: gaps
                    .iter()
                    .map(|(category, detail)| Blocker {
                        code: "incomplete_build".to_string(),
                        category: Some((*category).to_string()),
                        sku: None,
                        detail: (*detail).to_string(),
                    })
                    .collect(),
                adjustable: vec![
                    "Add a graphics card or cooler to the inventory.".to_string(),
                    "Record `igpu=yes` or `bundled_cooler=yes` on a CPU with its source, then re-run."
                        .to_string(),
                ],
                minimum_build_minor: None,
                budget_shortfall_minor: None,
            }),
            search: stats,
            exclusions,
        };
    }

    let minimum = minimum_complete_cost(pools, gpu_required);
    if let Some(minimum) = minimum {
        if minimum > settings.budget_minor {
            let shortfall = minimum - settings.budget_minor;
            let mut blockers = Vec::new();
            if gpu_required {
                if let Some(gpu) = cheapest_in(pools, "gpu") {
                    blockers.push(Blocker {
                        code: "use_case_requirement".to_string(),
                        category: Some("gpu".to_string()),
                        sku: Some(gpu.record.sku.clone()),
                        detail: format!(
                            "Gaming requires a discrete graphics card; the cheapest here is {} at {}.",
                            gpu.record.model_normalized,
                            format_money(gpu.line_total_minor, currency)
                        ),
                    });
                }
            }
            blockers.extend(required_cheapest_blockers(pools, currency));
            return SearchOutcome {
                candidates: Vec::new(),
                no_solution: Some(NoSolution {
                    binding: "budget".to_string(),
                    headline: if gpu_required {
                        "The budget is below the cheapest gaming build with a discrete graphics card."
                            .to_string()
                    } else {
                        "The budget is below the cheapest complete build.".to_string()
                    },
                    explanation: if gpu_required {
                        format!(
                            "Gaming requires a discrete graphics card, so a CPU with sourced `igpu=yes` alone is not eligible. The cheapest parts recorded in this snapshot that can form a complete gaming build with a card add up to at least {}, which is {} over the {} budget.",
                            format_money(minimum, currency),
                            format_money(shortfall, currency),
                            format_money(settings.budget_minor, currency)
                        )
                    } else {
                        format!(
                            "The cheapest parts recorded in this snapshot that can form a complete build add up to at least {}, which is {} over the {} budget.",
                            format_money(minimum, currency),
                            format_money(shortfall, currency),
                            format_money(settings.budget_minor, currency)
                        )
                    },
                    blockers,
                    adjustable: vec![
                        format!(
                            "Raise the budget to at least {}.",
                            format_money(minimum, currency)
                        ),
                        if gpu_required {
                            "Add a cheaper discrete graphics card, or exclude the parts that make the card unaffordable."
                                .to_string()
                        } else {
                            "Lower the cost by excluding expensive parts or importing cheaper stock."
                                .to_string()
                        },
                    ],
                    minimum_build_minor: Some(minimum),
                    budget_shortfall_minor: Some(shortfall),
                }),
                search: stats,
                exclusions,
            };
        }
    }

    let mut walker = Walker::new(snapshot, settings, pools);
    walker.walk(pools, 0, 0);
    stats.nodes_examined = walker.nodes;
    stats.leaves_evaluated = walker.leaves;
    stats.valid_builds = walker.valid;
    stats.limited = walker.limited;

    if walker.valid == 0 {
        let no_solution = if walker.limited {
            search_limit_solution(walker.nodes, walker.leaves, minimum)
        } else {
            compatibility_solution(walker.leaves, &walker.rejections, settings)
        };
        return SearchOutcome {
            candidates: Vec::new(),
            no_solution: Some(no_solution),
            search: stats,
            exclusions,
        };
    }

    let cheapest = walker.cheapest.clone();
    let best_performance = walker.best_performance.clone();
    let valid_builds = walker.valid;
    let mut pool: Vec<Rc<Evaluated>> = walker.top;
    for extra in [cheapest.clone(), best_performance.clone()]
        .into_iter()
        .flatten()
    {
        if !pool
            .iter()
            .any(|entry| entry.material_signature == extra.material_signature)
        {
            pool.push(extra);
        }
    }
    pool.sort_by(|left, right| {
        better_by_score(
            left.scores.total,
            left.total,
            &left.signature,
            right.scores.total,
            right.total,
            &right.signature,
        )
    });

    let mut ordered: Vec<(Rc<Evaluated>, &'static str, &'static str)> = Vec::new();
    let mut used: BTreeSet<String> = BTreeSet::new();
    if let Some(first) = pool.first() {
        used.insert(first.material_signature.clone());
        ordered.push((Rc::clone(first), "best_overall", "Best overall"));
    }
    for (candidate, objective, label) in [
        (cheapest.as_ref(), "lowest_cost", "Lowest cost"),
        (
            best_performance.as_ref(),
            "highest_performance",
            "Performance first",
        ),
    ] {
        let Some(candidate) = candidate else {
            continue;
        };
        if used.contains(&candidate.material_signature) {
            continue;
        }
        used.insert(candidate.material_signature.clone());
        ordered.push((Rc::clone(candidate), objective, label));
    }
    for entry in &pool {
        if ordered.len() >= MAX_CANDIDATES {
            break;
        }
        if used.contains(&entry.material_signature) {
            continue;
        }
        used.insert(entry.material_signature.clone());
        ordered.push((Rc::clone(entry), "alternative", "Alternative build"));
    }

    let candidates = ordered
        .into_iter()
        .enumerate()
        .map(|(index, (evaluated, objective, label))| {
            make_candidate(
                (*evaluated).clone(),
                index + 1,
                objective,
                label,
                pools,
                settings,
                valid_builds,
            )
        })
        .collect();

    SearchOutcome {
        candidates,
        no_solution: None,
        search: stats,
        exclusions,
    }
}
