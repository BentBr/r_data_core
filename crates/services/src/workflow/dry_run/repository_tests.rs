#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Behaviour tests for the dry-run overlay.
//!
//! The stub below stands in for the live database: it records nothing and
//! answers from a fixed set, so any write reaching it would be a bug the tests
//! can see.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value as JsonValue};
use uuid::Uuid;

use r_data_core_core::entity_definition::EntityDefinition;
use r_data_core_core::error::Result;
use r_data_core_core::DynamicEntity;
use r_data_core_persistence::{DynamicEntityRepositoryTrait, FilterEntitiesParams};

use super::repository::{DryRunEntityRepository, WriteKind};

/// Stands in for the live repository. Counts writes so a test can prove none
/// arrived, and serves a fixed row so fall-through reads are observable.
#[derive(Default)]
struct StubRepo {
    existing: Vec<DynamicEntity>,
    writes_received: AtomicUsize,
}

impl StubRepo {
    fn writes(&self) -> usize {
        self.writes_received.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl DynamicEntityRepositoryTrait for StubRepo {
    async fn get_all_by_type(
        &self,
        entity_type: &str,
        _limit: i64,
        _offset: i64,
        _exclusive_fields: Option<Vec<String>>,
    ) -> Result<Vec<DynamicEntity>> {
        Ok(self
            .existing
            .iter()
            .filter(|e| e.entity_type == entity_type)
            .cloned()
            .collect())
    }

    async fn get_by_type(
        &self,
        entity_type: &str,
        uuid: &Uuid,
        _exclusive_fields: Option<Vec<String>>,
    ) -> Result<Option<DynamicEntity>> {
        Ok(self
            .existing
            .iter()
            .find(|e| e.entity_type == entity_type && uuid_of(e) == *uuid)
            .cloned())
    }

    async fn create(&self, _entity: &DynamicEntity) -> Result<Uuid> {
        self.writes_received.fetch_add(1, Ordering::SeqCst);
        Ok(Uuid::now_v7())
    }

    async fn update(&self, _entity: &DynamicEntity) -> Result<()> {
        self.writes_received.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn delete_by_type(&self, _entity_type: &str, _uuid: &Uuid) -> Result<()> {
        self.writes_received.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn filter_entities(
        &self,
        entity_type: &str,
        _params: &FilterEntitiesParams,
    ) -> Result<Vec<DynamicEntity>> {
        Ok(self
            .existing
            .iter()
            .filter(|e| e.entity_type == entity_type)
            .cloned()
            .collect())
    }

    async fn count_entities(&self, entity_type: &str) -> Result<i64> {
        Ok(i64::try_from(
            self.existing
                .iter()
                .filter(|e| e.entity_type == entity_type)
                .count(),
        )
        .unwrap_or(0))
    }

    async fn count_children(&self, _parent_uuid: &Uuid) -> Result<i64> {
        Ok(0)
    }

    async fn get_by_uuid_any_type(&self, uuid: &Uuid) -> Result<Option<DynamicEntity>> {
        Ok(self.existing.iter().find(|e| uuid_of(e) == *uuid).cloned())
    }

    async fn find_one_by_filters(
        &self,
        entity_type: &str,
        _filters: &HashMap<String, JsonValue>,
    ) -> Result<Option<DynamicEntity>> {
        Ok(self
            .existing
            .iter()
            .find(|e| e.entity_type == entity_type)
            .cloned())
    }

    async fn get_raw_field_value(
        &self,
        _entity_type: &str,
        _uuid: &Uuid,
        _field_name: &str,
    ) -> Result<Option<String>> {
        Ok(Some("from-database".to_string()))
    }
}

fn uuid_of(e: &DynamicEntity) -> Uuid {
    e.field_data
        .get("uuid")
        .and_then(JsonValue::as_str)
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or_default()
}

fn entity(entity_type: &str, uuid: Uuid, name: &str) -> DynamicEntity {
    let mut field_data = HashMap::new();
    field_data.insert("uuid".to_string(), json!(uuid.to_string()));
    field_data.insert("name".to_string(), json!(name));
    DynamicEntity {
        entity_type: entity_type.to_string(),
        field_data,
        definition: Arc::new(EntityDefinition::default()),
    }
}

fn overlay_over(stub: Arc<StubRepo>) -> DryRunEntityRepository {
    DryRunEntityRepository::new(stub)
}

#[tokio::test]
async fn a_create_never_reaches_the_database() {
    let stub = Arc::new(StubRepo::default());
    let repo = overlay_over(Arc::clone(&stub));

    repo.create(&entity("customer", Uuid::now_v7(), "Ada"))
        .await
        .unwrap();

    assert_eq!(
        stub.writes(),
        0,
        "the live repository must never see a dry-run write"
    );
}

#[tokio::test]
async fn a_created_entity_is_visible_to_a_later_lookup() {
    // Read-your-writes is the whole reason for an overlay rather than a stub
    // that records and discards: step 2 must be able to resolve what step 1
    // created, exactly as it would in a real run.
    let stub = Arc::new(StubRepo::default());
    let repo = overlay_over(Arc::clone(&stub));
    let uuid = Uuid::now_v7();

    repo.create(&entity("customer", uuid, "Ada")).await.unwrap();

    let mut filters = HashMap::new();
    filters.insert("name".to_string(), json!("Ada"));
    let found = repo
        .find_one_by_filters("customer", &filters)
        .await
        .unwrap()
        .expect("the entity created moments ago must be findable");

    assert_eq!(found.field_data["name"], json!("Ada"));
}

#[tokio::test]
async fn a_created_entity_is_fetchable_by_uuid() {
    let stub = Arc::new(StubRepo::default());
    let repo = overlay_over(Arc::clone(&stub));
    let uuid = Uuid::now_v7();

    repo.create(&entity("customer", uuid, "Ada")).await.unwrap();

    assert!(repo
        .get_by_type("customer", &uuid, None)
        .await
        .unwrap()
        .is_some());
    assert!(repo.get_by_uuid_any_type(&uuid).await.unwrap().is_some());
}

#[tokio::test]
async fn reads_fall_through_to_the_database_when_the_overlay_is_empty() {
    let uuid = Uuid::now_v7();
    let stub = Arc::new(StubRepo {
        existing: vec![entity("customer", uuid, "Grace")],
        writes_received: AtomicUsize::new(0),
    });
    let repo = overlay_over(Arc::clone(&stub));

    let found = repo
        .get_by_type("customer", &uuid, None)
        .await
        .unwrap()
        .expect("existing rows must still be readable");
    assert_eq!(found.field_data["name"], json!("Grace"));
}

#[tokio::test]
async fn an_update_replaces_the_overlay_entry_rather_than_appending() {
    let stub = Arc::new(StubRepo::default());
    let repo = overlay_over(Arc::clone(&stub));
    let uuid = Uuid::now_v7();

    repo.create(&entity("customer", uuid, "Ada")).await.unwrap();
    repo.update(&entity("customer", uuid, "Ada Lovelace"))
        .await
        .unwrap();

    let found = repo
        .get_by_type("customer", &uuid, None)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        found.field_data["name"],
        json!("Ada Lovelace"),
        "a read must see the latest state, as it would mid-transaction"
    );
    assert_eq!(repo.count_entities("customer").await.unwrap(), 1);
}

#[tokio::test]
async fn a_delete_hides_the_entity_from_reads() {
    let uuid = Uuid::now_v7();
    let stub = Arc::new(StubRepo {
        existing: vec![entity("customer", uuid, "Grace")],
        writes_received: AtomicUsize::new(0),
    });
    let repo = overlay_over(Arc::clone(&stub));

    repo.delete_by_type("customer", &uuid).await.unwrap();

    assert!(
        repo.get_by_type("customer", &uuid, None)
            .await
            .unwrap()
            .is_none(),
        "a deleted entity must not be readable, even though the row still exists"
    );
    assert_eq!(stub.writes(), 0, "and the row must still exist");
}

#[tokio::test]
async fn counts_include_overlay_additions() {
    let stub = Arc::new(StubRepo {
        existing: vec![entity("customer", Uuid::now_v7(), "Grace")],
        writes_received: AtomicUsize::new(0),
    });
    let repo = overlay_over(Arc::clone(&stub));

    repo.create(&entity("customer", Uuid::now_v7(), "Ada"))
        .await
        .unwrap();

    assert_eq!(repo.count_entities("customer").await.unwrap(), 2);
}

#[tokio::test]
async fn every_suppressed_write_is_recorded_in_order() {
    let stub = Arc::new(StubRepo::default());
    let repo = overlay_over(Arc::clone(&stub));
    let uuid = Uuid::now_v7();

    repo.create(&entity("customer", uuid, "Ada")).await.unwrap();
    repo.update(&entity("customer", uuid, "Ada L"))
        .await
        .unwrap();
    repo.delete_by_type("customer", &uuid).await.unwrap();

    let writes = repo.recorded_writes().await;
    let kinds: Vec<WriteKind> = writes.iter().map(|w| w.kind).collect();
    assert_eq!(
        kinds,
        vec![WriteKind::Create, WriteKind::Update, WriteKind::Delete],
        "the trace reports what would have happened, in order"
    );
    assert!(writes.iter().all(|w| w.entity_type == "customer"));
}

#[tokio::test]
async fn filter_results_respect_the_requested_limit() {
    let stub = Arc::new(StubRepo {
        existing: vec![
            entity("customer", Uuid::now_v7(), "Grace"),
            entity("customer", Uuid::now_v7(), "Alan"),
        ],
        writes_received: AtomicUsize::new(0),
    });
    let repo = overlay_over(Arc::clone(&stub));
    repo.create(&entity("customer", Uuid::now_v7(), "Ada"))
        .await
        .unwrap();

    let params = FilterEntitiesParams::new(1, 0);
    let out = repo.filter_entities("customer", &params).await.unwrap();

    assert_eq!(out.len(), 1, "limit 1 must yield one row, got {out:?}");
}

#[tokio::test]
async fn a_raw_field_read_prefers_the_overlay() {
    // The authenticate transform reads raw fields. If it read the database
    // copy of an entity the dry-run just rewrote, it would authenticate
    // against stale data.
    let stub = Arc::new(StubRepo::default());
    let repo = overlay_over(Arc::clone(&stub));
    let uuid = Uuid::now_v7();
    repo.create(&entity("customer", uuid, "Ada")).await.unwrap();

    let got = repo
        .get_raw_field_value("customer", &uuid, "name")
        .await
        .unwrap();
    assert_eq!(got.as_deref(), Some("Ada"));
}
