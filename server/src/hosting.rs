//! 独立权限进程：只通过宿主受控通道读写用户、角色、部门与岗位，不接触数据库。

use anyhow::Result;
use axum::{Json, routing::get};
use az_plugin_process::Host;
use serde_json::{Value, json};
use std::sync::Arc;

/// 生产与真实 HTTP 验收使用同一宿主配置和受控入口。
pub async fn run() -> Result<()> {
    let host = Host::from_env()?;
    let identity = Arc::new(host.identity()?);
    let router = host
        .protect(crate::routes::router(identity))
        .route("/health", get(|| async { "ok" }))
        .route("/aio/describe", get(describe));
    host.serve(router).await
}

/// 页面随整包版本动态贡献；停用整包后入口一起移除。
pub async fn describe() -> Json<Value> {
    Json(json!({"label":"用户与权限","pages":[
        {
            "id":"users", "label":"用户管理", "entry":"index.html",
            "scene":["system","系统"], "menu_path":["系统管理"],
            "permission":"rbac:manage", "surface":"workspace"
        },
        {
            "id":"roles", "label":"角色管理", "entry":"index.html",
            "scene":["system","系统"], "menu_path":["系统管理"],
            "permission":"rbac:manage", "surface":"workspace"
        },
        {
            "id":"departments", "label":"部门管理", "entry":"index.html",
            "scene":["system","系统"], "menu_path":["系统管理"],
            "permission":"rbac:manage", "surface":"workspace"
        },
        {
            "id":"posts", "label":"岗位管理", "entry":"index.html",
            "scene":["system","系统"], "menu_path":["系统管理"],
            "permission":"rbac:manage", "surface":"workspace"
        }
    ]}))
}
