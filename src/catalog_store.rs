//! Catalog persistence: the dataset registry and dataset-scoped parts.
//!
//! Everything here runs inside the inventory's single SQLite connection. A
//! catalog is operator-supplied data, not inventory: deleting or replacing a
//! dataset touches only `catalog_datasets` and `catalog_parts`.

use std::collections::BTreeMap;

use rusqlite::{params, OptionalExtension, Row};
use uuid::Uuid;

use crate::catalog::{
    part_provenance, CatalogDataset, CatalogPart, StoredCatalogPart, ValidatedCatalog,
};
use crate::store::{Store, StoreError, StoreResult};

pub struct CatalogImportOutcome {
    pub dataset: CatalogDataset,
    pub replaced: bool,
}

fn dataset_from_row(row: &Row<'_>, offset: usize) -> rusqlite::Result<CatalogDataset> {
    let counts_json: String = row.get(offset + 9)?;
    let counts: BTreeMap<String, i64> = serde_json::from_str(&counts_json).unwrap_or_default();
    Ok(CatalogDataset {
        id: row.get(offset)?,
        name: row.get(offset + 1)?,
        source_url: row.get(offset + 2)?,
        commit: row.get(offset + 3)?,
        license: row.get(offset + 4)?,
        retrieved_at: row.get(offset + 5)?,
        imported_at: row.get(offset + 6)?,
        version: row.get(offset + 7)?,
        row_count: row.get(offset + 8)?,
        category_counts: counts,
        active: row.get::<_, i64>(offset + 10)? != 0,
    })
}

const DATASET_COLUMNS: &str = "id, name, source_url, commit_ref, license, retrieved_at,
     imported_at, version_label, row_count, category_counts_json, active";

const DATASET_COLUMNS_ALIASED: &str = "d.id, d.name, d.source_url, d.commit_ref, d.license,
     d.retrieved_at, d.imported_at, d.version_label, d.row_count,
     d.category_counts_json, d.active";

const PART_COLUMNS: &str = "p.id, p.dataset_id, p.external_id, p.sku, p.category, p.model,
     p.model_normalized, p.manufacturer, p.specs_json, p.source_url, p.verified_at";

impl Store {
    pub fn import_catalog(
        &self,
        catalog: &ValidatedCatalog,
        imported_at: &str,
    ) -> StoreResult<CatalogImportOutcome> {
        let version = catalog.version_label(imported_at);
        let counts_json = serde_json::to_string(&catalog.category_counts)?;
        let row_count = catalog.parts.len() as i64;
        self.with_connection(|connection| {
            let transaction = connection.transaction()?;
            let existing: Option<(String, i64)> = transaction
                .query_row(
                    "SELECT id, active FROM catalog_datasets WHERE name = ?1",
                    params![catalog.name],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            let replaced = existing.is_some();
            let id = existing
                .as_ref()
                .map(|(id, _)| id.clone())
                .unwrap_or_else(|| Uuid::new_v4().to_string());
            let active = existing.as_ref().map(|(_, active)| *active).unwrap_or(1);
            if replaced {
                transaction.execute(
                    "UPDATE catalog_datasets SET source_url = ?1, commit_ref = ?2,
                        license = ?3, retrieved_at = ?4, imported_at = ?5,
                        version_label = ?6, row_count = ?7, category_counts_json = ?8
                     WHERE id = ?9",
                    params![
                        catalog.source_url,
                        catalog.commit,
                        catalog.license,
                        catalog.retrieved_at,
                        imported_at,
                        version,
                        row_count,
                        counts_json,
                        id,
                    ],
                )?;
            } else {
                transaction.execute(
                    "INSERT INTO catalog_datasets
                        (id, name, source_url, commit_ref, license, retrieved_at,
                         imported_at, version_label, row_count, category_counts_json, active)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        id,
                        catalog.name,
                        catalog.source_url,
                        catalog.commit,
                        catalog.license,
                        catalog.retrieved_at,
                        imported_at,
                        version,
                        row_count,
                        counts_json,
                        active,
                    ],
                )?;
            }
            transaction.execute(
                "DELETE FROM catalog_parts WHERE dataset_id = ?1",
                params![id],
            )?;
            let dataset = CatalogDataset {
                id: id.clone(),
                name: catalog.name.clone(),
                source_url: catalog.source_url.clone(),
                commit: catalog.commit.clone(),
                license: catalog.license.clone(),
                retrieved_at: catalog.retrieved_at.clone(),
                imported_at: imported_at.to_string(),
                version: version.clone(),
                row_count,
                category_counts: catalog.category_counts.clone(),
                active: active != 0,
            };
            {
                let mut statement = transaction.prepare(
                    "INSERT INTO catalog_parts
                        (id, dataset_id, position, external_id, sku, category, model,
                         model_normalized, manufacturer, specs_json, source_url, verified_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                )?;
                for (position, part) in catalog.parts.iter().enumerate() {
                    let part_id = Uuid::new_v4().to_string();
                    let stored_part = CatalogPart {
                        id: part_id.clone(),
                        dataset_id: id.clone(),
                        external_id: part.external_id.clone(),
                        sku: part.sku.clone(),
                        category: part.category.clone(),
                        model: part.model.clone(),
                        model_normalized: part.model_normalized.clone(),
                        manufacturer: part.manufacturer.clone(),
                        specs: part.specs.clone(),
                        source_url: part.source_url.clone(),
                        verified_at: part.verified_at.clone(),
                    };
                    let provenance = part_provenance(&stored_part, &dataset);
                    let mut specs = part.specs.clone();
                    for spec in &mut specs {
                        spec.provenance = provenance.clone();
                    }
                    statement.execute(params![
                        part_id,
                        id,
                        position as i64,
                        part.external_id,
                        part.sku,
                        part.category,
                        part.model,
                        part.model_normalized,
                        part.manufacturer,
                        serde_json::to_string(&specs)?,
                        part.source_url,
                        part.verified_at,
                    ])?;
                }
            }
            transaction.commit()?;
            Ok(CatalogImportOutcome { dataset, replaced })
        })
    }

    pub fn list_catalog_datasets(&self) -> StoreResult<Vec<CatalogDataset>> {
        self.with_connection(|connection| {
            let mut statement = connection.prepare(&format!(
                "SELECT {DATASET_COLUMNS} FROM catalog_datasets ORDER BY name"
            ))?;
            let rows = statement
                .query_map([], |row| dataset_from_row(row, 0))?
                .collect::<Result<Vec<_>, rusqlite::Error>>()?;
            Ok(rows)
        })
    }

    pub fn get_catalog_dataset(&self, id: &str) -> StoreResult<CatalogDataset> {
        self.with_connection(|connection| {
            connection
                .query_row(
                    &format!("SELECT {DATASET_COLUMNS} FROM catalog_datasets WHERE id = ?1"),
                    params![id],
                    |row| dataset_from_row(row, 0),
                )
                .optional()?
                .ok_or(StoreError::NotFound)
        })
    }

    pub fn set_catalog_dataset_active(
        &self,
        id: &str,
        active: bool,
    ) -> StoreResult<CatalogDataset> {
        self.with_connection(|connection| {
            let changed = connection.execute(
                "UPDATE catalog_datasets SET active = ?1 WHERE id = ?2",
                params![i64::from(active), id],
            )?;
            if changed == 0 {
                return Err(StoreError::NotFound);
            }
            Ok(())
        })?;
        self.get_catalog_dataset(id)
    }

    /// Remove a dataset and its parts. No inventory, snapshot, check,
    /// recommendation or diagnosis row references a catalog part.
    pub fn delete_catalog_dataset(&self, id: &str) -> StoreResult<()> {
        self.with_connection(|connection| {
            let changed =
                connection.execute("DELETE FROM catalog_datasets WHERE id = ?1", params![id])?;
            if changed == 0 {
                Err(StoreError::NotFound)
            } else {
                Ok(())
            }
        })
    }

    pub fn get_catalog_part(&self, id: &str) -> StoreResult<StoredCatalogPart> {
        self.with_connection(|connection| {
            connection
                .query_row(
                    &format!(
                        "SELECT {PART_COLUMNS}, {DATASET_COLUMNS_ALIASED}
                         FROM catalog_parts p JOIN catalog_datasets d ON d.id = p.dataset_id
                         WHERE p.id = ?1"
                    ),
                    params![id],
                    stored_part_from_row,
                )
                .optional()?
                .ok_or(StoreError::NotFound)
        })
    }

    pub fn get_catalog_parts(&self, ids: &[String]) -> StoreResult<Vec<StoredCatalogPart>> {
        let mut parts = Vec::new();
        for id in ids {
            parts.push(self.get_catalog_part(id)?);
        }
        Ok(parts)
    }

    /// Parts from active datasets, optionally scoped to one canonical category.
    /// Ordering is stable so ranking and matching never depend on row order.
    pub fn active_catalog_parts(
        &self,
        category: Option<&str>,
    ) -> StoreResult<Vec<StoredCatalogPart>> {
        self.with_connection(|connection| {
            let sql = match category {
                Some(_) => format!(
                    "SELECT {PART_COLUMNS}, {DATASET_COLUMNS_ALIASED}
                     FROM catalog_parts p JOIN catalog_datasets d ON d.id = p.dataset_id
                     WHERE d.active = 1 AND p.category = ?1
                     ORDER BY p.category, p.sku, p.model, p.id"
                ),
                None => format!(
                    "SELECT {PART_COLUMNS}, {DATASET_COLUMNS_ALIASED}
                     FROM catalog_parts p JOIN catalog_datasets d ON d.id = p.dataset_id
                     WHERE d.active = 1
                     ORDER BY p.category, p.sku, p.model, p.id"
                ),
            };
            let mut statement = connection.prepare(&sql)?;
            let parts = match category {
                Some(category) => statement
                    .query_map(params![category], stored_part_from_row)?
                    .collect::<Result<Vec<_>, rusqlite::Error>>()?,
                None => statement
                    .query_map([], stored_part_from_row)?
                    .collect::<Result<Vec<_>, rusqlite::Error>>()?,
            };
            Ok(parts)
        })
    }
}

fn stored_part_from_row(row: &Row<'_>) -> rusqlite::Result<StoredCatalogPart> {
    let specs_json: String = row.get(8)?;
    let specs = serde_json::from_str(&specs_json).unwrap_or_default();
    Ok(StoredCatalogPart {
        part: CatalogPart {
            id: row.get(0)?,
            dataset_id: row.get(1)?,
            external_id: row.get(2)?,
            sku: row.get(3)?,
            category: row.get(4)?,
            model: row.get(5)?,
            model_normalized: row.get(6)?,
            manufacturer: row.get(7)?,
            specs,
            source_url: row.get(9)?,
            verified_at: row.get(10)?,
        },
        dataset: dataset_from_row(row, 11)?,
    })
}
