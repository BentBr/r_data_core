#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Guided workflows for the two things people actually ask for.
//!
//! The loop is the product. Left to invent one, a model tends to draft a
//! program from remembered syntax, save it, and discover the problems by
//! running it against real data — which is exactly the experience this server
//! exists to replace. Encoding the sequence costs two prompts and removes that
//! whole failure mode.

use rmcp::model::{PromptMessage, Role};
use rmcp::{prompt, prompt_router, ErrorData};

use crate::tools::RdcTools;

/// The authoring sequence, in the order it has to happen.
const AUTHOR_WORKFLOW: &str = "\
To author an RDataCore workflow, follow this sequence. Steps 4 and 5 are the \
ones worth insisting on.

1. Call `dsl_options` for the parts of the step you need — `from`, `to`, \
   `transform`. It is generated from the running engine, so it is accurate for \
   this instance in a way no documentation can be. Do not write DSL from \
   remembered syntax.

2. If the workflow reads from or writes to entities, call \
   `get_entity_definition` for each type involved and use the real field \
   names. Guessing them is the single most common cause of a workflow that \
   validates and then fails.

3. Draft the program. Read `rdatacore://dsl/validation-rules` if you are \
   unsure about step ordering or type casting.

4. Call `validate_dsl`. Fix whatever it reports — it names the exact JSON path \
   and, for an unknown variant, the legal alternatives — and validate again.

5. Call `test_workflow` with representative sample input. Read the per-step \
   trace and confirm each step produced what you expected. This executes for \
   real against an in-memory overlay and persists nothing, so iterate here \
   freely rather than guessing.

6. Call `create_workflow`. It stays disabled.

7. Call `run_workflow` once to confirm it works against real data.

8. Call `update_workflow` with `enabled: true`.

Never skip 4 and 5. Validation catches structural errors; only the dry-run \
catches the mapping and type errors that appear at runtime, and those are the \
ones that would otherwise be discovered by writing bad data.";

/// The debugging sequence.
const DEBUG_FAILING_RUN: &str = "\
To debug a failing RDataCore workflow run:

1. Call `list_runs` — with `status: \"failed\"` if you are looking for a recent \
   failure — and note the run's UUID.

2. Call `get_run_logs` for that run. Read the `meta` field: it usually names \
   the step that failed, which saves reading the whole program.

3. Call `get_workflow` to read the current DSL program.

4. Locate the failing step. Check its construct against `dsl_options` for that \
   part of the step, paying particular attention to field names in mappings \
   and to the type-casting rules in `rdatacore://dsl/validation-rules`.

5. Propose a fix and confirm it with `test_workflow`, using input resembling \
   what the failing run received. No side effects, so iterate here.

6. Apply it with `update_workflow`.

7. Call `run_workflow` to verify.

Two things worth knowing. If the run failed on a type cast, the fix is \
usually in the transform's operands or the from-mapping, not in the \
destination. And if a previous version worked, `list_workflow_versions` and \
`restore_workflow_version` are faster than reconstructing the program — \
restoring appends a new version rather than rewinding, so nothing is lost.";

#[prompt_router(vis = "pub")]
impl RdcTools {
    /// Author a workflow, from discovery through to enabling it.
    #[prompt(
        name = "author_workflow",
        description = "The sequence for writing a new RDataCore workflow: discover the \
                       vocabulary and the target entity, draft, validate, dry-run, then \
                       save disabled and enable only once tested."
    )]
    pub async fn author_workflow_prompt(&self) -> Result<Vec<PromptMessage>, ErrorData> {
        Ok(vec![PromptMessage::new_text(Role::User, AUTHOR_WORKFLOW)])
    }

    /// Find and fix a failing run.
    #[prompt(
        name = "debug_failing_run",
        description = "The sequence for diagnosing a failed workflow run: find it, read \
                       its logs, locate the failing step, and confirm a fix with a \
                       dry-run before applying it."
    )]
    pub async fn debug_failing_run_prompt(&self) -> Result<Vec<PromptMessage>, ErrorData> {
        Ok(vec![PromptMessage::new_text(Role::User, DEBUG_FAILING_RUN)])
    }
}

#[cfg(test)]
mod tests;
