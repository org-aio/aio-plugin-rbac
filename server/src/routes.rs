//! 权限 HTTP 入口：身份与权限由宿主注入，写操作仍会经过宿主受控身份服务复核。

use aio_plugin_rbac_model::{
    AccessControlErrorResponse, AccessControlResponse, AccessControlView, AssignRoleRequest,
    CreateRoleRequest, CreateUserRequest, OrganizationItem, ResetMemberPasswordRequest,
    SaveDepartmentRequest, SavePostRequest, UpdateMemberRequest, UpdateMemberRolesRequest,
};
use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use az_plugin_process::{Caller, IdentityService};
use std::sync::Arc;

type Service = Arc<IdentityService>;

pub fn router(identity: Service) -> Router {
    Router::new()
        .route("/rbac", get(view))
        .route("/rbac/users", post(create_user))
        .route("/rbac/users/{id}", put(update_member).delete(remove_member))
        .route("/rbac/users/{id}/roles", put(set_member_roles))
        .route("/rbac/users/{id}/password", post(reset_member_password))
        .route("/rbac/roles", post(create_role))
        .route("/rbac/roles/{id}", put(update_role).delete(delete_role))
        .route("/rbac/departments", post(create_department))
        .route(
            "/rbac/departments/{id}",
            put(update_department).delete(delete_department),
        )
        .route("/rbac/posts", post(create_post))
        .route("/rbac/posts/{id}", put(update_post).delete(delete_post))
        .route("/rbac/assignments", post(assign))
        .with_state(identity)
}

async fn view(
    State(identity): State<Service>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<AccessControlResponse<AccessControlView>>, RbacError> {
    require_manage(&caller)?;
    let view = identity
        .access_control_view(&caller.context_id)
        .await
        .map_err(RbacError::from_identity)?;
    Ok(Json(AccessControlResponse {
        data: to_view(view),
    }))
}

async fn create_user(
    State(identity): State<Service>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<CreateUserRequest>,
) -> Result<StatusCode, RbacError> {
    require_manage(&caller)?;
    let user_id = identity
        .create_user(
            &caller.context_id,
            &request.account,
            &request.display_name,
            &request.password,
        )
        .await
        .map_err(RbacError::from_identity)?;
    // 新账号加入当前租户并授予 `member` 角色，与既有产品语义一致。
    identity
        .set_member_roles(&caller.context_id, &user_id, &["member".to_owned()])
        .await
        .map_err(RbacError::from_identity)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_role(
    State(identity): State<Service>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<CreateRoleRequest>,
) -> Result<StatusCode, RbacError> {
    require_manage(&caller)?;
    identity
        .save_role(
            &caller.context_id,
            &request.role_id,
            &request.permissions,
            true,
        )
        .await
        .map_err(RbacError::from_identity)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_role(
    State(identity): State<Service>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
    Json(request): Json<CreateRoleRequest>,
) -> Result<StatusCode, RbacError> {
    require_manage(&caller)?;
    if id != request.role_id {
        return Err(RbacError::forbidden("角色 ID 不可更改"));
    }
    identity
        .save_role(&caller.context_id, &id, &request.permissions, false)
        .await
        .map_err(RbacError::from_identity)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_role(
    State(identity): State<Service>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
) -> Result<StatusCode, RbacError> {
    require_manage(&caller)?;
    identity
        .delete_role(&caller.context_id, &id)
        .await
        .map_err(RbacError::from_identity)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_member(
    State(identity): State<Service>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
    Json(request): Json<UpdateMemberRequest>,
) -> Result<StatusCode, RbacError> {
    require_manage(&caller)?;
    identity
        .update_member(&caller.context_id, &id, &request.display_name)
        .await
        .map_err(RbacError::from_identity)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove_member(
    State(identity): State<Service>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
) -> Result<StatusCode, RbacError> {
    require_manage(&caller)?;
    identity
        .remove_member(&caller.context_id, &id)
        .await
        .map_err(RbacError::from_identity)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn set_member_roles(
    State(identity): State<Service>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
    Json(request): Json<UpdateMemberRolesRequest>,
) -> Result<StatusCode, RbacError> {
    require_manage(&caller)?;
    identity
        .set_member_roles(&caller.context_id, &id, &request.roles)
        .await
        .map_err(RbacError::from_identity)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn reset_member_password(
    State(identity): State<Service>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
    Json(request): Json<ResetMemberPasswordRequest>,
) -> Result<StatusCode, RbacError> {
    require_manage(&caller)?;
    // 先由权限提供者裁决授权范围，再由身份服务写入新密码。
    identity
        .authorize_member_password_reset(&caller.context_id, &id)
        .await
        .map_err(RbacError::from_identity)?;
    identity
        .reset_password(&caller.context_id, &id, &request.new_password)
        .await
        .map_err(RbacError::from_identity)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn assign(
    State(identity): State<Service>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<AssignRoleRequest>,
) -> Result<StatusCode, RbacError> {
    require_manage(&caller)?;
    identity
        .set_member_roles(&caller.context_id, &request.user_id, &[request.role_id])
        .await
        .map_err(RbacError::from_identity)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_department(
    State(identity): State<Service>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<SaveDepartmentRequest>,
) -> Result<StatusCode, RbacError> {
    require_manage(&caller)?;
    identity
        .save_department(&caller.context_id, to_department(request))
        .await
        .map_err(RbacError::from_identity)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_department(
    State(identity): State<Service>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
    Json(mut request): Json<SaveDepartmentRequest>,
) -> Result<StatusCode, RbacError> {
    require_manage(&caller)?;
    request.id = Some(id);
    identity
        .save_department(&caller.context_id, to_department(request))
        .await
        .map_err(RbacError::from_identity)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_department(
    State(identity): State<Service>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
) -> Result<StatusCode, RbacError> {
    require_manage(&caller)?;
    identity
        .delete_department(&caller.context_id, &id)
        .await
        .map_err(RbacError::from_identity)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_post(
    State(identity): State<Service>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<SavePostRequest>,
) -> Result<StatusCode, RbacError> {
    require_manage(&caller)?;
    identity
        .save_post(&caller.context_id, to_post(request))
        .await
        .map_err(RbacError::from_identity)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_post(
    State(identity): State<Service>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
    Json(mut request): Json<SavePostRequest>,
) -> Result<StatusCode, RbacError> {
    require_manage(&caller)?;
    request.id = Some(id);
    identity
        .save_post(&caller.context_id, to_post(request))
        .await
        .map_err(RbacError::from_identity)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_post(
    State(identity): State<Service>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
) -> Result<StatusCode, RbacError> {
    require_manage(&caller)?;
    identity
        .delete_post(&caller.context_id, &id)
        .await
        .map_err(RbacError::from_identity)?;
    Ok(StatusCode::NO_CONTENT)
}

fn require_manage(caller: &Caller) -> Result<(), RbacError> {
    if caller.has_permission("rbac:manage") {
        Ok(())
    } else {
        Err(RbacError::forbidden("当前角色没有权限管理权限"))
    }
}

/// 契约视图与插件模型字段一致，转换只做类型搬运。
fn to_view(
    view: az_plugin_process::az_plugin_contract::process::AccessControlView,
) -> AccessControlView {
    AccessControlView {
        users: view
            .users
            .into_iter()
            .map(|user| aio_plugin_rbac_model::UserItem {
                id: user.id,
                account: user.account,
                display_name: user.display_name,
                roles: user.roles,
            })
            .collect(),
        roles: view
            .roles
            .into_iter()
            .map(|role| aio_plugin_rbac_model::RoleItem {
                id: role.id,
                permissions: role.permissions,
                member_count: role.member_count,
            })
            .collect(),
        departments: view.departments.into_iter().map(to_org).collect(),
        posts: view.posts.into_iter().map(to_org).collect(),
        current_user_id: view.current_user_id,
        grantable_permissions: view.grantable_permissions,
    }
}

fn to_org(
    item: az_plugin_process::az_plugin_contract::process::AccessControlOrganization,
) -> OrganizationItem {
    OrganizationItem {
        id: item.id,
        code: item.code,
        name: item.name,
        parent_id: item.parent_id,
        leader: item.leader,
        phone: item.phone,
        email: item.email,
        sort_order: item.sort_order,
        status: item.status,
        remark: item.remark,
    }
}

fn to_department(
    value: SaveDepartmentRequest,
) -> az_plugin_process::az_plugin_contract::process::AccessControlDepartment {
    az_plugin_process::az_plugin_contract::process::AccessControlDepartment {
        id: value.id,
        parent_id: value.parent_id,
        name: value.name,
        leader: value.leader,
        phone: value.phone,
        email: value.email,
        sort_order: value.sort_order,
        status: value.status,
    }
}

fn to_post(
    value: SavePostRequest,
) -> az_plugin_process::az_plugin_contract::process::AccessControlPost {
    az_plugin_process::az_plugin_contract::process::AccessControlPost {
        id: value.id,
        code: value.code,
        name: value.name,
        sort_order: value.sort_order,
        status: value.status,
        remark: value.remark,
    }
}

struct RbacError {
    status: StatusCode,
    message: String,
}

impl RbacError {
    fn forbidden(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message: message.into(),
        }
    }

    /// 身份服务的失败大多来自业务校验；权限与越权保留 403 语义。
    fn from_identity(error: anyhow::Error) -> Self {
        let message = format!("{error:#}");
        let status = if message.contains("权限")
            || message.contains("不能")
            || message.contains("只有")
            || message.contains("不属于")
        {
            StatusCode::FORBIDDEN
        } else {
            StatusCode::BAD_REQUEST
        };
        Self { status, message }
    }
}

impl IntoResponse for RbacError {
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
