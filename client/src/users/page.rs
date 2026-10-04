use super::dialogs::{UserEditor, UserPassword, UserRoles};
use crate::http;
use aio_plugin_rbac_model::UserItem;
use az_ui_components::{
    admin::{
        AsyncResult, CollectionTable, DeleteRecordsDialog, PageHeader, PageSurface, RequestState,
        SortValue, StatusMessage,
    },
    badge::{Badge, BadgeVariant},
    button::{Button, ButtonSize, ButtonVariant},
    data_table::{DataTableCellContext, DataTableColumn},
};
use dioxus::prelude::*;
use dioxus_icons::lucide::{KeyRound, Pencil, Plus, RefreshCw, Shield, UserMinus};

#[allow(non_snake_case)]
pub(crate) fn UsersPage() -> Element {
    let mut revision = use_signal(|| 0_u64);
    let access = use_resource(move || {
        let _ = revision();
        http::load()
    });
    let mut editor = use_signal(|| None::<Option<UserItem>>);
    let mut assignments = use_signal(|| None::<UserItem>);
    let mut resets = use_signal(|| None::<UserItem>);
    let mut removal = use_signal(|| None::<Vec<UserItem>>);
    let mut selected = use_signal(Vec::<UserItem>::new);
    let mut feedback = use_signal(|| None::<String>);
    let data = match access.read().as_ref().cloned() {
        Some(Ok(value)) => value,
        Some(Err(error)) => {
            return rsx! { PageSurface { RequestState { error, on_retry: move |_| revision += 1 } } };
        }
        None => return rsx! { PageSurface { RequestState {} } },
    };
    let actor = data.current_user_id;
    rsx! {
        PageSurface {
            PageHeader { title: "用户管理", detail: format!("当前租户 · {} 位用户", data.users.len()),
                Button { size: ButtonSize::Icon, variant: ButtonVariant::Outline, title: "刷新用户", aria_label: "刷新用户", onclick: move |_| revision += 1, RefreshCw {} }
                Button { onclick: move |_| editor.set(Some(None)), Plus {} "新建用户" }
            }
            if let Some(message) = feedback() { StatusMessage { message } }
            CollectionTable { label: "用户", rows: data.users,
                columns: vec![DataTableColumn::leaf("name", "显示名称").width(200), DataTableColumn::leaf("account", "账号").width(180), DataTableColumn::leaf("roles", "角色").width(340), DataTableColumn::leaf("actions", "操作").width(190)],
                row_key: |user: UserItem| user.id, search_text: |user: UserItem| format!("{} {} {}", user.display_name, user.account, user.roles.join(" ")),
                sort_value: |(user, key): (UserItem, String)| SortValue::Text(if key == "account" { user.account } else { user.display_name }), sortable: vec!["name".into(), "account".into()],
                selected_keys: selected().iter().map(|u| u.id.clone()).collect::<std::collections::BTreeSet<_>>(), on_selection_change: move |items| selected.set(items),
                tools: rsx! { if !selected().is_empty() { Button { variant: ButtonVariant::Outline, onclick: move |_| removal.set(Some(selected())), UserMinus {} "移出选中 ({selected().len()})" } } },
                render_cell: move |context: DataTableCellContext<UserItem>| {
                    let user = context.row;
                    match context.column.key.as_str() {
                        "name" => rsx! { "{user.display_name}" }, "account" => rsx! { code { class: "admin-code", "{user.account}" } },
                        "roles" => rsx! { div { class: "admin-badges", for role in &user.roles { Badge { variant: BadgeVariant::Outline, "{role}" } } } },
                        "actions" => { let edit = user.clone(); let roles = user.clone(); let reset = user.clone(); let is_self = user.id == actor; rsx! { div { class: "admin-actions",
                            Button { size: ButtonSize::IconSm, variant: ButtonVariant::Ghost, title: "编辑 {user.account}", aria_label: "编辑 {user.account}", onclick: move |_| editor.set(Some(Some(edit.clone()))), Pencil {} }
                            Button { size: ButtonSize::IconSm, variant: ButtonVariant::Ghost, title: "重置密码 {user.account}", aria_label: "重置密码 {user.account}", onclick: move |_| resets.set(Some(reset.clone())), KeyRound {} }
                            Button { size: ButtonSize::IconSm, variant: ButtonVariant::Ghost, title: "分配角色 {user.account}", aria_label: "分配角色 {user.account}", onclick: move |_| assignments.set(Some(roles.clone())), Shield {} }
                            Button { size: ButtonSize::IconSm, variant: ButtonVariant::Ghost, disabled: is_self, title: "移出 {user.account}", aria_label: "移出 {user.account}", onclick: move |_| removal.set(Some(vec![user.clone()])), UserMinus {} }
                        } } }, _ => rsx! {},
                    }
                },
            }
        }
        if let Some(value) = editor() { UserEditor { value, on_close: move |_| editor.set(None), on_saved: move |_| { editor.set(None); feedback.set(Some("用户已保存".into())); revision += 1; } } }
        if let Some(value) = resets() { UserPassword { value, on_close: move |_| resets.set(None), on_saved: move |_| { resets.set(None); feedback.set(Some("密码已重置，该账号原有会话已失效".into())); revision += 1; } } }
        if let Some(value) = assignments() { UserRoles { value, roles: data.roles, on_close: move |_| assignments.set(None), on_saved: move |_| { assignments.set(None); feedback.set(Some("角色分配已更新".into())); revision += 1; } } }
        if let Some(items) = removal() { DeleteRecordsDialog { title: "移出租户", warning: "移除当前租户成员关系并结束其会话，保留平台账号和其他租户数据。", items,
            item_label: |user: UserItem| format!("{} (@{})", user.display_name, user.account), delete: |user: UserItem| -> AsyncResult<()> { Box::pin(async move { http::delete(&format!("/api/rbac/users/{}", user.id)).await }) },
            on_close: move |_| removal.set(None), on_deleted: move |count| { selected.set(Vec::new()); feedback.set(Some(format!("已移出 {count} 位用户"))); revision += 1; },
        } }
    }
}
