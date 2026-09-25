use std::sync::Arc;

use aio_plugin_identity_server::{IdentityService, SessionContext};
use aio_plugin_rbac_model::{
    AccessControlErrorResponse, AccessControlResponse, AccessControlView, AssignRoleRequest,
    CreateRoleRequest, CreateUserRequest, SaveDepartmentRequest, SavePostRequest,
    UpdateMemberRequest, UpdateMemberRolesRequest,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};

use crate::AccessControlService;

#[derive(Clone)]
struct AccessControlState {
    access: Arc<AccessControlService>,
    identity: Arc<IdentityService>,
}

pub fn router(access: Arc<AccessControlService>, identity: Arc<IdentityService>) -> Router {
    Router::new()
        .route("/api/plugins/rbac/health", get(health))
        .route("/api/rbac", get(view))
        .route("/api/rbac/users", post(create_user))
        .route(
            "/api/rbac/users/{id}",
            put(update_member).delete(remove_member),
        )
        .route("/api/rbac/users/{id}/roles", put(set_member_roles))
        .route("/api/rbac/roles", post(create_role))
        .route("/api/rbac/roles/{id}", put(update_role).delete(delete_role))
        .route("/api/rbac/departments", post(create_department))
        .route(
            "/api/rbac/departments/{id}",
            put(update_department).delete(delete_department),
        )
        .route("/api/rbac/posts", post(create_post))
        .route("/api/rbac/posts/{id}", put(update_post).delete(delete_post))
        .route("/api/rbac/assignments", post(assign))
        .with_state(AccessControlState { access, identity })
}

async fn health() -> &'static str {
    "ok"
}

async fn view(
    State(state): State<AccessControlState>,
    headers: HeaderMap,
) -> Result<Json<AccessControlResponse<AccessControlView>>, AccessControlHttpError> {
    let session = authenticate_manager(&state, &headers).await?;
    Ok(Json(AccessControlResponse {
        data: state
            .access
            .view(&session.tenant_id, &session.user_id)
            .await?,
    }))
}

async fn create_user(
    State(state): State<AccessControlState>,
    headers: HeaderMap,
    Json(request): Json<CreateUserRequest>,
) -> Result<StatusCode, AccessControlHttpError> {
    let session = authenticate_manager(&state, &headers).await?;
    let user_id = state
        .identity
        .create_user(
            &session.tenant_id,
            &request.account,
            &request.display_name,
            &request.password,
        )
        .await?;
    state
        .access
        .set_member_roles(
            &session.tenant_id,
            &session.user_id,
            &user_id,
            &["member".into()],
            true,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_role(
    State(state): State<AccessControlState>,
    headers: HeaderMap,
    Json(request): Json<CreateRoleRequest>,
) -> Result<StatusCode, AccessControlHttpError> {
    let session = authenticate_manager(&state, &headers).await?;
    state
        .access
        .save_role(
            &session.tenant_id,
            &session.user_id,
            &request.role_id,
            &request.permissions,
            true,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_department(
    State(state): State<AccessControlState>,
    headers: HeaderMap,
    Json(request): Json<SaveDepartmentRequest>,
) -> Result<StatusCode, AccessControlHttpError> {
    let session = authenticate_manager(&state, &headers).await?;
    state
        .access
        .save_department(&session.tenant_id, &session.user_id, request)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_department(
    State(state): State<AccessControlState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(mut request): Json<SaveDepartmentRequest>,
) -> Result<StatusCode, AccessControlHttpError> {
    let session = authenticate_manager(&state, &headers).await?;
    request.id = Some(id);
    state
        .access
        .save_department(&session.tenant_id, &session.user_id, request)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_department(
    State(state): State<AccessControlState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, AccessControlHttpError> {
    let session = authenticate_manager(&state, &headers).await?;
    state
        .access
        .delete_department(&session.tenant_id, &session.user_id, &id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_post(
    State(state): State<AccessControlState>,
    headers: HeaderMap,
    Json(request): Json<SavePostRequest>,
) -> Result<StatusCode, AccessControlHttpError> {
    let session = authenticate_manager(&state, &headers).await?;
    state
        .access
        .save_post(&session.tenant_id, &session.user_id, request)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_post(
    State(state): State<AccessControlState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(mut request): Json<SavePostRequest>,
) -> Result<StatusCode, AccessControlHttpError> {
    let session = authenticate_manager(&state, &headers).await?;
    request.id = Some(id);
    state
        .access
        .save_post(&session.tenant_id, &session.user_id, request)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_post(
    State(state): State<AccessControlState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, AccessControlHttpError> {
    let session = authenticate_manager(&state, &headers).await?;
    state
        .access
        .delete_post(&session.tenant_id, &session.user_id, &id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn assign(
    State(state): State<AccessControlState>,
    headers: HeaderMap,
    Json(request): Json<AssignRoleRequest>,
) -> Result<StatusCode, AccessControlHttpError> {
    let session = authenticate_manager(&state, &headers).await?;
    state
        .access
        .set_member_roles(
            &session.tenant_id,
            &session.user_id,
            &request.user_id,
            &[request.role_id],
            true,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_member(
    State(state): State<AccessControlState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<UpdateMemberRequest>,
) -> Result<StatusCode, AccessControlHttpError> {
    let session = authenticate_manager(&state, &headers).await?;
    state
        .access
        .update_member(
            &session.tenant_id,
            &session.user_id,
            &id,
            &request.display_name,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove_member(
    State(state): State<AccessControlState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, AccessControlHttpError> {
    let session = authenticate_manager(&state, &headers).await?;
    state
        .access
        .remove_member(&session.tenant_id, &session.user_id, &id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn set_member_roles(
    State(state): State<AccessControlState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<UpdateMemberRolesRequest>,
) -> Result<StatusCode, AccessControlHttpError> {
    let session = authenticate_manager(&state, &headers).await?;
    state
        .access
        .set_member_roles(
            &session.tenant_id,
            &session.user_id,
            &id,
            &request.roles,
            false,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_role(
    State(state): State<AccessControlState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<CreateRoleRequest>,
) -> Result<StatusCode, AccessControlHttpError> {
    let session = authenticate_manager(&state, &headers).await?;
    if id != request.role_id {
        return Err(AccessControlHttpError::forbidden("角色 ID 不可更改"));
    }
    state
        .access
        .save_role(
            &session.tenant_id,
            &session.user_id,
            &id,
            &request.permissions,
            false,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_role(
    State(state): State<AccessControlState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, AccessControlHttpError> {
    let session = authenticate_manager(&state, &headers).await?;
    state
        .access
        .delete_role(&session.tenant_id, &session.user_id, &id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn authenticate_manager(
    state: &AccessControlState,
    headers: &HeaderMap,
) -> Result<SessionContext, AccessControlHttpError> {
    let session = state
        .identity
        .authenticate(headers)
        .await?
        .ok_or_else(|| AccessControlHttpError::unauthorized("会话无效或已过期"))?;
    if !session
        .permissions
        .iter()
        .any(|permission| permission == "rbac:manage")
    {
        return Err(AccessControlHttpError::forbidden(
            "当前角色没有权限管理权限",
        ));
    }
    Ok(session)
}

struct AccessControlHttpError {
    status: StatusCode,
    message: String,
}

impl AccessControlHttpError {
    fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: message.into(),
        }
    }

    fn forbidden(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message: message.into(),
        }
    }
}

impl<E> From<E> for AccessControlHttpError
where
    E: Into<anyhow::Error>,
{
    fn from(value: E) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: format!("{:#}", value.into()),
        }
    }
}

impl IntoResponse for AccessControlHttpError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(AccessControlErrorResponse {
                error: self.message,
            }),
        )
            .into_response()
    }
}
