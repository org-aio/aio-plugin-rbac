use super::dialog::RoleEditor;
use crate::http;
use aio_plugin_rbac_model::{RoleItem, is_system_role, permission_label};
use az_ui_components::{
    admin::{
        AsyncResult, CollectionTable, DeleteRecordsDialog, PageHeader, PageSurface, RequestState,
        SortValue, StatusMessage,
    },
    badge::{Badge, BadgeVariant},
    button::{Button, ButtonSize, ButtonVariant},
    data_table::{DataTableAlign, DataTableCellContext, DataTableColumn},
};
use dioxus::prelude::*;
use dioxus_icons::lucide::{LockKeyhole, Pencil, Plus, RefreshCw, Trash2};

#[allow(non_snake_case)]
pub(crate) fn RolesPage() -> Element {
    let mut revision = use_signal(|| 0_u64);
    let access = use_resource(move || {
        let _ = revision();
        http::load()
    });
    let mut editor = use_signal(|| None::<Option<RoleItem>>);
    let mut removal = use_signal(|| None::<RoleItem>);
    let mut feedback = use_signal(|| None::<String>);
    let data = match access.read().as_ref().cloned() {
        Some(Ok(value)) => value,
        Some(Err(error)) => {
            return rsx! { PageSurface { RequestState { error, on_retry: move |_| revision += 1 } } };
        }
        None => return rsx! { PageSurface { RequestState {} } },
    };
    rsx! {
        PageSurface {
            PageHeader { title: "角色管理", detail: format!("当前租户 · {} 个角色", data.roles.len()),
                Button { size: ButtonSize::Icon, variant: ButtonVariant::Outline, title: "刷新角色", aria_label: "刷新角色", onclick: move |_| revision += 1, RefreshCw {} }
                Button { onclick: move |_| editor.set(Some(None)), Plus {} "新建角色" }
            }
            if let Some(message) = feedback() { StatusMessage { message } }
            p { class: "admin-meta", "已安装插件的权限由租户所有成员自动获得，无需在角色中配置。" }
            CollectionTable { label: "角色", rows: data.roles,
                columns: vec![DataTableColumn::leaf("id", "角色 ID").width(240), DataTableColumn::leaf("permissions", "权限").width(430), DataTableColumn::leaf("members", "用户数").width(100).align(DataTableAlign::End), DataTableColumn::leaf("actions", "操作").width(110)],
                row_key: |role: RoleItem| role.id,
                search_text: |role: RoleItem| format!("{} {}", role.id, role.permissions.iter().map(|p| format!("{p} {}", permission_label(p))).collect::<Vec<_>>().join(" ")),
                sort_value: |(role, key): (RoleItem, String)| if key == "members" { SortValue::Number(role.member_count as i128) } else { SortValue::Text(role.id) }, sortable: vec!["id".into(), "members".into()],
                render_cell: move |context: DataTableCellContext<RoleItem>| {
                    let role = context.row;
                    match context.column.key.as_str() {
                        "id" => rsx! { div { class: "admin-actions", code { class: "admin-code", "{role.id}" } if is_system_role(&role.id) { Badge { variant: BadgeVariant::Outline, "内置" } } } },
                        "permissions" => rsx! { div { class: "admin-badges", for permission in role.permissions { Badge { variant: BadgeVariant::Outline, title: permission_label(&permission), "{permission_label(&permission)}" } } } },
                        "members" => rsx! { "{role.member_count}" },
                        "actions" => { let edit = role.clone(); rsx! { div { class: "admin-actions",
                            if is_system_role(&role.id) { span { title: "内置角色只读", aria_label: "内置角色只读", LockKeyhole {} } }
                            else {
                                Button { size: ButtonSize::IconSm, variant: ButtonVariant::Ghost, title: "编辑 {role.id}", aria_label: "编辑 {role.id}", onclick: move |_| editor.set(Some(Some(edit.clone()))), Pencil {} }
                                Button { size: ButtonSize::IconSm, variant: ButtonVariant::Ghost, title: "删除 {role.id}", aria_label: "删除 {role.id}", onclick: move |_| removal.set(Some(role.clone())), Trash2 {} }
                            }
                        } } }, _ => rsx! {},
                    }
                },
            }
        }
        if let Some(value) = editor() { RoleEditor { value, permissions: data.grantable_permissions, on_close: move |_| editor.set(None), on_saved: move |_| { editor.set(None); feedback.set(Some("角色已保存".into())); revision += 1; } } }
        if let Some(value) = removal() { DeleteRecordsDialog { title: "删除角色", items: vec![value], item_label: |role: RoleItem| role.id,
            delete: |role: RoleItem| -> AsyncResult<()> { Box::pin(async move { http::delete(&format!("/api/rbac/roles/{}", role.id)).await }) },
            on_close: move |_| removal.set(None), on_deleted: move |_| { feedback.set(Some("角色已删除".into())); revision += 1; },
        } }
    }
}
