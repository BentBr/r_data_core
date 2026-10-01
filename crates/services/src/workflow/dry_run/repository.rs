#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! A `DynamicEntityRepositoryTrait` that reads the real database but writes
//! only to memory.
//!
//! This is what makes a dry-run both faithful and safe. `DslProgram::execute`
//! is pure but incomplete — it skips the four async transforms — so a trace
//! built from it would be confidently wrong for any program using them. The
//! dry-run therefore runs the *real* executor, and gets its safety here
//! instead: reads fall through to the live repository, writes land in an
//! overlay that subsequent reads consult.
//!
//! Read-your-writes is the point. A step that creates an entity and a later
//! step that resolves it by path both behave exactly as they would in
//! production, which a record-and-discard stub cannot reproduce.
//!
//! **What this deliberately does not do:** the database is never touched, so
//! database-level constraints are never exercised. A dry-run can pass where a
//! real run fails on a unique or foreign-key violation. That was a conscious
//! trade against threading a transaction through shared persistence code; the
//! trait seam means a transaction-backed variant can replace this later
//! without any caller changing.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use tokio::sync::Mutex;
use uuid::Uuid;

use r_data_core_core::error::Result;
use r_data_core_core::DynamicEntity;
use r_data_core_persistence::{DynamicEntityRepositoryTrait, FilterEntitiesParams};

/// A write the dry-run suppressed, kept so the trace can report it.
#[derive(Debug, Clone)]
pub struct RecordedWrite {
    pub kind: WriteKind,
    pub entity_type: String,
    pub uuid: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteKind {
    Create,
    Update,
    Delete,
}

/// Entities written during the dry-run, plus a log of what was written.
#[derive(Default)]
struct Overlay {
    /// Live entities by `(entity_type, uuid)`. Updates replace in place, so a
    /// read always sees the latest state — as it would mid-transaction.
    entities: HashMap<(String, Uuid), DynamicEntity>,
    /// Deleted keys, so a read after a delete correctly finds nothing.
    deleted: Vec<(String, Uuid)>,
    /// Keys the dry-run created, as opposed to updated.
    ///
    /// Needed to keep counts and listings honest: an update puts an entity in
    /// the overlay that also exists in the database, and treating every
    /// overlay entry as an addition would count it twice and list it twice —
    /// once stale from the database, once from here.
    created: Vec<(String, Uuid)>,
    writes: Vec<RecordedWrite>,
}

/// Reads through to the real repository; writes stay in memory.
pub struct DryRunEntityRepository {
    inner: Arc<dyn DynamicEntityRepositoryTrait + Send + Sync>,
    overlay: Mutex<Overlay>,
}

impl DryRunEntityRepository {
    #[must_use]
    pub fn new(inner: Arc<dyn DynamicEntityRepositoryTrait + Send + Sync>) -> Self {
        Self {
            inner,
            overlay: Mutex::new(Overlay::default()),
        }
    }

    /// Every write the dry-run suppressed, in the order it was attempted.
    pub async fn recorded_writes(&self) -> Vec<RecordedWrite> {
        self.overlay.lock().await.writes.clone()
    }

    /// The uuid an entity carries in its field data, if any.
    ///
    /// Entities are keyed by uuid throughout, and `DynamicEntity::new` seeds
    /// one, so a missing or unparseable uuid means the caller built the entity
    /// by hand. Generating one keeps the overlay usable rather than dropping
    /// the write.
    fn uuid_of(entity: &DynamicEntity) -> Uuid {
        entity
            .field_data
            .get("uuid")
            .and_then(JsonValue::as_str)
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::now_v7)
    }

    /// Whether an entity satisfies every filter, comparing as the repository
    /// does for equality lookups.
    ///
    /// Filters often arrive as strings even when the stored value is a number
    /// or bool — the SQL comparison casts, so this does too. It parses the
    /// filter rather than stringifying the stored value, which avoids an
    /// allocation per comparison on what can be a hot path.
    fn matches(entity: &DynamicEntity, filters: &HashMap<String, JsonValue>) -> bool {
        filters.iter().all(|(field, want)| {
            entity.field_data.get(field).is_some_and(|got| {
                if got == want {
                    return true;
                }
                want.as_str().is_some_and(|w| match got {
                    JsonValue::String(s) => s == w,
                    JsonValue::Number(n) => n
                        .as_f64()
                        .zip(w.parse::<f64>().ok())
                        .is_some_and(|(a, b)| (a - b).abs() < f64::EPSILON),
                    JsonValue::Bool(b) => w.parse::<bool>().is_ok_and(|p| p == *b),
                    _ => false,
                })
            })
        })
    }

    async fn is_deleted(&self, entity_type: &str, uuid: &Uuid) -> bool {
        self.overlay
            .lock()
            .await
            .deleted
            .iter()
            .any(|(t, u)| t == entity_type && u == uuid)
    }
}

#[async_trait]
impl DynamicEntityRepositoryTrait for DryRunEntityRepository {
    async fn get_all_by_type(
        &self,
        entity_type: &str,
        limit: i64,
        offset: i64,
        exclusive_fields: Option<Vec<String>>,
    ) -> Result<Vec<DynamicEntity>> {
        let mut out = self
            .inner
            .get_all_by_type(entity_type, limit, offset, exclusive_fields)
            .await?;

        let (overlaid, deleted) = {
            let overlay = self.overlay.lock().await;
            let overlaid: Vec<(Uuid, DynamicEntity)> = overlay
                .entities
                .iter()
                .filter(|((t, _), _)| t == entity_type)
                .map(|((_, uuid), e)| (*uuid, e.clone()))
                .collect();
            let deleted: Vec<Uuid> = overlay
                .deleted
                .iter()
                .filter(|(t, _)| t == entity_type)
                .map(|(_, uuid)| *uuid)
                .collect();
            // Released before the merging below, which needs no lock.
            drop(overlay);
            (overlaid, deleted)
        };

        // A row the dry-run deleted must not still be listed.
        out.retain(|e| !deleted.contains(&Self::uuid_of(e)));
        // An updated row must appear once, with the new values — not twice,
        // once stale from the database and once from the overlay.
        for (uuid, entity) in overlaid {
            if let Some(existing) = out.iter_mut().find(|e| Self::uuid_of(e) == uuid) {
                *existing = entity;
            } else {
                out.push(entity);
            }
        }
        Ok(out)
    }

    async fn get_by_type(
        &self,
        entity_type: &str,
        uuid: &Uuid,
        exclusive_fields: Option<Vec<String>>,
    ) -> Result<Option<DynamicEntity>> {
        if self.is_deleted(entity_type, uuid).await {
            return Ok(None);
        }
        let hit = {
            let overlay = self.overlay.lock().await;
            overlay
                .entities
                .get(&(entity_type.to_string(), *uuid))
                .cloned()
        };
        if let Some(found) = hit {
            return Ok(Some(found));
        }
        self.inner
            .get_by_type(entity_type, uuid, exclusive_fields)
            .await
    }

    async fn create(&self, entity: &DynamicEntity) -> Result<Uuid> {
        let uuid = Self::uuid_of(entity);
        {
            let mut overlay = self.overlay.lock().await;
            let key = (entity.entity_type.clone(), uuid);
            overlay.entities.insert(key.clone(), entity.clone());
            if !overlay.created.contains(&key) {
                overlay.created.push(key);
            }
            overlay.writes.push(RecordedWrite {
                kind: WriteKind::Create,
                entity_type: entity.entity_type.clone(),
                uuid,
            });
        }
        Ok(uuid)
    }

    async fn update(&self, entity: &DynamicEntity) -> Result<()> {
        let uuid = Self::uuid_of(entity);
        {
            let mut overlay = self.overlay.lock().await;
            overlay
                .entities
                .insert((entity.entity_type.clone(), uuid), entity.clone());
            overlay.writes.push(RecordedWrite {
                kind: WriteKind::Update,
                entity_type: entity.entity_type.clone(),
                uuid,
            });
        }
        Ok(())
    }

    async fn delete_by_type(&self, entity_type: &str, uuid: &Uuid) -> Result<()> {
        {
            let mut overlay = self.overlay.lock().await;
            overlay.entities.remove(&(entity_type.to_string(), *uuid));
            overlay.deleted.push((entity_type.to_string(), *uuid));
            overlay.writes.push(RecordedWrite {
                kind: WriteKind::Delete,
                entity_type: entity_type.to_string(),
                uuid: *uuid,
            });
        }
        Ok(())
    }

    async fn filter_entities(
        &self,
        entity_type: &str,
        params: &FilterEntitiesParams,
    ) -> Result<Vec<DynamicEntity>> {
        // Overlay matches come first: a step that just created an entity must
        // find it, and the real query cannot see it.
        let empty = HashMap::new();
        let filters = params.filters.as_ref().unwrap_or(&empty);
        let mut out: Vec<DynamicEntity> = {
            let overlay = self.overlay.lock().await;
            overlay
                .entities
                .iter()
                .filter(|((t, _), e)| t == entity_type && Self::matches(e, filters))
                .map(|(_, e)| e.clone())
                .collect()
        };

        if i64::try_from(out.len()).unwrap_or(i64::MAX) < params.limit {
            out.extend(self.inner.filter_entities(entity_type, params).await?);
        }
        out.truncate(usize::try_from(params.limit.max(0)).unwrap_or(usize::MAX));
        Ok(out)
    }

    async fn count_entities(&self, entity_type: &str) -> Result<i64> {
        let real = self.inner.count_entities(entity_type).await?;
        let (created, removed) = {
            let overlay = self.overlay.lock().await;
            // A row created and then deleted in the same run nets to nothing,
            // so it counts as neither an addition nor a removal.
            let created = overlay
                .created
                .iter()
                .filter(|key| key.0 == entity_type && !overlay.deleted.contains(key))
                .count();
            // Deleting a pre-existing row reduces the real count; deleting one
            // this run created does not, since it was never in that count.
            let removed = overlay
                .deleted
                .iter()
                .filter(|key| key.0 == entity_type && !overlay.created.contains(key))
                .count();
            drop(overlay);
            (created, removed)
        };
        Ok(real + i64::try_from(created).unwrap_or(0) - i64::try_from(removed).unwrap_or(0))
    }

    async fn count_children(&self, parent_uuid: &Uuid) -> Result<i64> {
        self.inner.count_children(parent_uuid).await
    }

    async fn get_by_uuid_any_type(&self, uuid: &Uuid) -> Result<Option<DynamicEntity>> {
        let hit = {
            let overlay = self.overlay.lock().await;
            overlay
                .entities
                .iter()
                .find(|((_, u), _)| u == uuid)
                .map(|(_, e)| e.clone())
        };
        if let Some(found) = hit {
            return Ok(Some(found));
        }
        self.inner.get_by_uuid_any_type(uuid).await
    }

    async fn find_one_by_filters(
        &self,
        entity_type: &str,
        filters: &HashMap<String, JsonValue>,
    ) -> Result<Option<DynamicEntity>> {
        let hit = {
            let overlay = self.overlay.lock().await;
            overlay
                .entities
                .iter()
                .find(|((t, _), e)| t == entity_type && Self::matches(e, filters))
                .map(|(_, e)| e.clone())
        };
        if let Some(found) = hit {
            return Ok(Some(found));
        }
        self.inner.find_one_by_filters(entity_type, filters).await
    }

    async fn get_raw_field_value(
        &self,
        entity_type: &str,
        uuid: &Uuid,
        field_name: &str,
    ) -> Result<Option<String>> {
        let hit = {
            let overlay = self.overlay.lock().await;
            overlay
                .entities
                .get(&(entity_type.to_string(), *uuid))
                .cloned()
        };
        if let Some(found) = hit {
            return Ok(found.field_data.get(field_name).map(|v| {
                v.as_str()
                    .map_or_else(|| v.to_string(), std::string::ToString::to_string)
            }));
        }
        self.inner
            .get_raw_field_value(entity_type, uuid, field_name)
            .await
    }
}
