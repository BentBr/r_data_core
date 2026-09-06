#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Running a DSL program for real, against an overlay, and reporting what it did.

use std::sync::Arc;

use serde_json::{json, Value as JsonValue};
use uuid::Uuid;

use r_data_core_core::dto::dsl::{DryRunResponse, DryRunStepTrace, StepEffect};
use r_data_core_core::error::Result;
use r_data_core_persistence::{DynamicEntityRepositoryTrait, WorkflowRepositoryTrait};
use r_data_core_workflow::dsl::{DslProgram, OutputMode, ToDef, Transform};

use super::repository::{DryRunEntityRepository, WriteKind};
use crate::dynamic_entity::DynamicEntityService;
use crate::workflow::item_processing::{WorkflowItemContext, WorkflowPipelineExecutor};
use crate::workflow::outbox::PushDispatchMode;
use crate::workflow::output_handling::WorkflowOutputDispatcher;
use crate::workflow::transform_execution::{JwtConfig, MailContext};

/// What the dry-run needs from the application in order to run a program.
pub struct DryRunDeps<'a> {
    /// The live entity service, when the deployment has one. Its repository is
    /// wrapped in an overlay, so reads reach the database and writes never do.
    ///
    /// `None` is legitimate: a program of pure transforms needs no entity
    /// infrastructure to dry-run. Steps that would need it are reported as
    /// unavailable rather than quietly doing nothing.
    pub entity_service: Option<&'a DynamicEntityService>,
    /// Needed for run-log writes on the executor's error path.
    pub workflow_repository: &'a Arc<dyn WorkflowRepositoryTrait>,
    pub jwt_secret: Option<&'a str>,
    pub jwt_expiration: u64,
}

/// Execute `program` against `input` without changing anything.
///
/// Entity reads and writes go through [`DryRunEntityRepository`], so mappings,
/// path templates and lookups are all exercised for real and later steps see
/// what earlier ones produced. Email and outbound pushes are never attempted:
/// no overlay can un-send them.
///
/// # Errors
/// Returns an error if the program is invalid or a step fails. A failure is
/// itself useful output — it is what the caller is trying to discover — and
/// costs nothing, because nothing was persisted.
pub async fn execute_dry_run(
    program: &DslProgram,
    input: &JsonValue,
    deps: DryRunDeps<'_>,
) -> Result<DryRunResponse> {
    program.validate()?;

    let overlay = deps.entity_service.map(|live| {
        Arc::new(DryRunEntityRepository::new(Arc::clone(
            live.get_repository(),
        )))
    });
    let entity_service = overlay
        .as_ref()
        .zip(deps.entity_service)
        .map(|(overlay, live)| {
            DynamicEntityService::new(
                Arc::clone(overlay) as Arc<dyn DynamicEntityRepositoryTrait + Send + Sync>,
                live.entity_definition_service(),
            )
        });

    let jwt = JwtConfig {
        secret: deps.jwt_secret,
        expiration: deps.jwt_expiration,
    };
    // No mail service and no queue: `handle_send_email` short-circuits rather
    // than sending, which is exactly the suppression a dry-run needs.
    let mail = MailContext {
        service: None,
        queue: None,
    };

    let ctx = WorkflowItemContext {
        dynamic_entity_service: entity_service.as_ref(),
        // Irrelevant in practice: push outputs are never dispatched below.
        push_dispatch: PushDispatchMode::Direct,
        repo: deps.workflow_repository,
        jwt: &jwt,
        mail: &mail,
        workflow_name: Some("dry-run"),
        // A dry-run writes no entities, so there is nothing to version.
        versioning_disabled: true,
    };

    let run_uuid = Uuid::now_v7();
    let executor = WorkflowPipelineExecutor::new(program, Uuid::now_v7(), run_uuid, &ctx, true);

    // `execute_inline` runs every step, including the async transforms, and
    // returns the outputs without dispatching them. Dispatching is our choice
    // to make, per target.
    let outputs = executor.execute_inline(input).await?;

    let dispatcher = WorkflowOutputDispatcher::new(&ctx);
    let item_uuid = Uuid::now_v7();
    let mut steps = Vec::with_capacity(outputs.len());

    let entities_available = entity_service.is_some();
    for (step_index, (to_def, produced)) in outputs.into_iter().enumerate() {
        let effect = classify(&to_def, entities_available);

        // Entity writes are dispatched: they land in the overlay, so the
        // mapping and path template are genuinely exercised and a later step
        // can read the result.
        if matches!(to_def, ToDef::Entity { .. }) && entity_service.is_some() {
            dispatcher
                .handle_entity_output(&to_def, &produced, input, item_uuid, run_uuid)
                .await?;
        }

        steps.push(DryRunStepTrace {
            step_index,
            suppressed_detail: suppressed_detail(&to_def, &produced, entities_available),
            produced: scrub_suppression_markers(program, step_index, produced),
            to: serde_json::to_value(&to_def).unwrap_or(JsonValue::Null),
            effect,
        });
    }

    let recorded = match overlay.as_ref() {
        Some(overlay) => overlay.recorded_writes().await,
        None => Vec::new(),
    };
    let would_write = recorded
        .into_iter()
        .map(|w| {
            let verb = match w.kind {
                WriteKind::Create => "create",
                WriteKind::Update => "update",
                WriteKind::Delete => "delete",
            };
            format!("{verb} {} {}", w.entity_type, w.uuid)
        })
        .collect();

    Ok(DryRunResponse { steps, would_write })
}

/// How honest the dry-run can be about a given target.
const fn classify(to_def: &ToDef, entities_available: bool) -> StepEffect {
    match to_def {
        // Dispatched against the overlay, so genuinely proven — but only when
        // there is an entity service to dispatch through.
        ToDef::Entity { .. } if entities_available => StepEffect::Executed,
        // Everything else that touches the outside world is described rather
        // than performed: an entity write with no service to dispatch through,
        // and email and pushes, which no overlay could undo.
        ToDef::Entity { .. }
        | ToDef::Email { .. }
        | ToDef::Format {
            output: OutputMode::Push { .. },
            ..
        } => StepEffect::Suppressed,
        _ => StepEffect::NoSideEffect,
    }
}

/// A human-readable note about what a suppressed target would have done.
fn suppressed_detail(
    to_def: &ToDef,
    produced: &JsonValue,
    entities_available: bool,
) -> Option<String> {
    match to_def {
        ToDef::Entity {
            entity_definition, ..
        } if !entities_available => Some(format!(
            "would write a {entity_definition}, but this deployment has no dynamic \
             entity service so the step could not be exercised"
        )),
        ToDef::Email { template_uuid, .. } => {
            Some(format!("would send email using template {template_uuid}"))
        }
        ToDef::Format {
            output: OutputMode::Push { destination, .. },
            ..
        } => Some(format!(
            "would push {} to {}",
            summarise(produced),
            serde_json::to_value(destination)
                .ok()
                .and_then(|v| v.get("uri").and_then(|u| u.as_str()).map(str::to_string))
                .unwrap_or_else(|| "the configured destination".to_string())
        )),
        _ => None,
    }
}

fn summarise(produced: &JsonValue) -> String {
    produced.as_object().map_or_else(
        || "the step output".to_string(),
        |o| format!("{} field(s)", o.len()),
    )
}

/// Replace the marker `handle_send_email` leaves when no mailer is configured.
///
/// With mail deliberately unset, that path writes `mail_not_configured` into
/// the normalized data. In a dry-run that reads as a configuration problem
/// rather than intentional suppression, which would send someone off checking
/// SMTP settings that are perfectly fine.
fn scrub_suppression_markers(
    program: &DslProgram,
    step_index: usize,
    mut produced: JsonValue,
) -> JsonValue {
    let Some(step) = program.steps.get(step_index) else {
        return produced;
    };
    let Transform::SendEmail(se) = &step.transform else {
        return produced;
    };
    if let Some(obj) = produced.as_object_mut() {
        if obj.get(&se.target_status).and_then(JsonValue::as_str) == Some("mail_not_configured") {
            obj.insert(se.target_status.clone(), json!("suppressed_in_dry_run"));
        }
    }
    produced
}
