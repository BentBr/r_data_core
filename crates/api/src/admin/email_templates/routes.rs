#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{delete, get, post, put, web, Responder};
use uuid::Uuid;

use crate::admin::email_templates::models::{
    CreateEmailTemplateRequest, EmailTemplateListQuery, EmailTemplateResponse,
    UpdateEmailTemplateRequest,
};
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::RequiredAuth;
use crate::auth::permission_check;
use crate::response::ApiResponse;
use r_data_core_core::email_template::EmailTemplateType;
use r_data_core_core::permissions::role::{PermissionType, ResourceNamespace};
use r_data_core_core::system_log::SystemLogResourceType;
use r_data_core_persistence::{EmailTemplateRepository, EmailTemplateRepositoryTrait};

#[utoipa::path(
    get,
    path = "/admin/api/v1/email-templates",
    tag = "email-templates",
    params(
        ("type" = Option<String>, Query, description = "Filter by type: 'system' or 'workflow'")
    ),
    responses(
        (status = 200, description = "List of email templates", body = [EmailTemplateResponse]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Server error")
    ),
    security(("jwt" = []))
)]
#[get("")]
pub async fn list_email_templates(
    data: web::Data<ApiStateWrapper>,
    query: web::Query<EmailTemplateListQuery>,
    auth: RequiredAuth,
) -> impl Responder {
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::System,
        &PermissionType::Read,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to view email templates");
    }

    let repo = EmailTemplateRepository::new(data.db_pool().clone());

    let result = match &query.template_type {
        Some(t) => repo.list_by_type(t.clone()).await,
        None => repo.list_all().await,
    };

    match result {
        Ok(templates) => {
            let dtos: Vec<EmailTemplateResponse> = templates
                .into_iter()
                .map(EmailTemplateResponse::from)
                .collect();
            ApiResponse::ok(dtos)
        }
        Err(e) => {
            log::error!("Failed to list email templates: {e}");
            ApiResponse::<()>::internal_error("Failed to list email templates")
        }
    }
}

#[utoipa::path(
    get,
    path = "/admin/api/v1/email-templates/{uuid}",
    tag = "email-templates",
    params(("uuid" = Uuid, Path, description = "Email template UUID")),
    responses(
        (status = 200, description = "Email template", body = EmailTemplateResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Server error")
    ),
    security(("jwt" = []))
)]
#[get("/{uuid}")]
pub async fn get_email_template(
    data: web::Data<ApiStateWrapper>,
    path: web::Path<Uuid>,
    auth: RequiredAuth,
) -> impl Responder {
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::System,
        &PermissionType::Read,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to view email templates");
    }

    let uuid = path.into_inner();
    let repo = EmailTemplateRepository::new(data.db_pool().clone());

    match repo.get_by_uuid(uuid).await {
        Ok(Some(t)) => ApiResponse::ok(EmailTemplateResponse::from(t)),
        Ok(None) => ApiResponse::<()>::not_found("Email template not found"),
        Err(e) => {
            log::error!("Failed to get email template {uuid}: {e}");
            ApiResponse::<()>::internal_error("Failed to get email template")
        }
    }
}

#[utoipa::path(
    post,
    path = "/admin/api/v1/email-templates",
    tag = "email-templates",
    request_body = CreateEmailTemplateRequest,
    responses(
        (status = 201, description = "Created"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 409, description = "Conflict - slug already in use"),
        (status = 500, description = "Server error")
    ),
    security(("jwt" = []))
)]
#[post("")]
pub async fn create_email_template(
    data: web::Data<ApiStateWrapper>,
    body: web::Json<CreateEmailTemplateRequest>,
    auth: RequiredAuth,
) -> impl Responder {
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::System,
        &PermissionType::Create,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to create email templates");
    }

    let Some(created_by) = auth.user_uuid() else {
        return ApiResponse::<()>::internal_error("No authentication claims found");
    };

    let repo = EmailTemplateRepository::new(data.db_pool().clone());

    // Check slug uniqueness
    match repo.get_by_slug(&body.slug).await {
        Ok(Some(_)) => {
            return ApiResponse::<()>::conflict("A template with this slug already exists");
        }
        Ok(None) => {}
        Err(e) => {
            log::error!("Failed to check email template slug: {e}");
            return ApiResponse::<()>::internal_error("Failed to check slug uniqueness");
        }
    }

    match repo
        .create(
            &body.name,
            &body.slug,
            EmailTemplateType::Workflow,
            &body.subject_template,
            &body.body_html_template,
            &body.body_text_template,
            body.variables.clone(),
            created_by,
        )
        .await
    {
        Ok(uuid) => {
            if let Some(log_svc) = data.system_log_service() {
                log_svc
                    .log_entity_created(
                        Some(created_by),
                        SystemLogResourceType::EmailTemplate,
                        uuid,
                        &format!("Email template '{}' created", body.name),
                        Some(serde_json::json!({"name": body.name, "slug": body.slug})),
                    )
                    .await;
            }
            ApiResponse::<serde_json::Value>::created(serde_json::json!({ "uuid": uuid }))
        }
        Err(e) => {
            log::error!("Failed to create email template: {e}");
            ApiResponse::<()>::internal_error("Failed to create email template")
        }
    }
}

#[utoipa::path(
    put,
    path = "/admin/api/v1/email-templates/{uuid}",
    tag = "email-templates",
    params(("uuid" = Uuid, Path, description = "Email template UUID")),
    request_body = UpdateEmailTemplateRequest,
    responses(
        (status = 200, description = "Updated"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Server error")
    ),
    security(("jwt" = []))
)]
#[put("/{uuid}")]
pub async fn update_email_template(
    data: web::Data<ApiStateWrapper>,
    path: web::Path<Uuid>,
    body: web::Json<UpdateEmailTemplateRequest>,
    auth: RequiredAuth,
) -> impl Responder {
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::System,
        &PermissionType::Update,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to update email templates");
    }

    let Some(updated_by) = auth.user_uuid() else {
        return ApiResponse::<()>::internal_error("No authentication claims found");
    };

    let uuid = path.into_inner();
    let repo = EmailTemplateRepository::new(data.db_pool().clone());

    // Fetch template to enforce type-based restrictions
    let template = match repo.get_by_uuid(uuid).await {
        Ok(Some(t)) => t,
        Ok(None) => return ApiResponse::<()>::not_found("Email template not found"),
        Err(e) => {
            log::error!("Failed to fetch email template {uuid}: {e}");
            return ApiResponse::<()>::internal_error("Failed to fetch email template");
        }
    };

    // System templates: name must not change
    let effective_name = if template.template_type == EmailTemplateType::System {
        None // COALESCE will keep existing name
    } else {
        body.name.as_deref()
    };

    match repo
        .update(
            uuid,
            effective_name,
            &body.subject_template,
            &body.body_html_template,
            &body.body_text_template,
            body.variables.clone(),
            updated_by,
        )
        .await
    {
        Ok(()) => {
            if let Some(log_svc) = data.system_log_service() {
                log_svc
                    .log_entity_updated(
                        Some(updated_by),
                        SystemLogResourceType::EmailTemplate,
                        uuid,
                        &format!("Email template '{}' updated", template.name),
                        Some(serde_json::json!({"name": template.name})),
                    )
                    .await;
            }
            ApiResponse::<()>::message("Updated")
        }
        Err(e) => {
            log::error!("Failed to update email template {uuid}: {e}");
            ApiResponse::<()>::internal_error("Failed to update email template")
        }
    }
}

#[utoipa::path(
    delete,
    path = "/admin/api/v1/email-templates/{uuid}",
    tag = "email-templates",
    params(("uuid" = Uuid, Path, description = "Email template UUID")),
    responses(
        (status = 200, description = "Deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden — cannot delete system templates"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Server error")
    ),
    security(("jwt" = []))
)]
#[delete("/{uuid}")]
pub async fn delete_email_template(
    data: web::Data<ApiStateWrapper>,
    path: web::Path<Uuid>,
    auth: RequiredAuth,
) -> impl Responder {
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::System,
        &PermissionType::Delete,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to delete email templates");
    }

    let uuid = path.into_inner();
    let repo = EmailTemplateRepository::new(data.db_pool().clone());

    // Fetch template to enforce system-template protection
    let template = match repo.get_by_uuid(uuid).await {
        Ok(Some(t)) => t,
        Ok(None) => return ApiResponse::<()>::not_found("Email template not found"),
        Err(e) => {
            log::error!("Failed to fetch email template {uuid}: {e}");
            return ApiResponse::<()>::internal_error("Failed to fetch email template");
        }
    };

    if template.template_type == EmailTemplateType::System {
        return ApiResponse::<()>::forbidden("System email templates cannot be deleted");
    }

    match repo.delete(uuid).await {
        Ok(()) => {
            if let Some(log_svc) = data.system_log_service() {
                let actor = auth.user_uuid();
                log_svc
                    .log_entity_deleted(
                        actor,
                        SystemLogResourceType::EmailTemplate,
                        uuid,
                        &format!("Email template '{}' deleted", template.name),
                        Some(serde_json::json!({"name": template.name})),
                    )
                    .await;
            }
            ApiResponse::<()>::message("Deleted")
        }
        Err(e) => {
            log::error!("Failed to delete email template {uuid}: {e}");
            ApiResponse::<()>::internal_error("Failed to delete email template")
        }
    }
}

/// Register email template routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_email_templates)
        .service(get_email_template)
        .service(create_email_template)
        .service(update_email_template)
        .service(delete_email_template);
}
