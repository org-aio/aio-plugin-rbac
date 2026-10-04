use crate::http;
use aio_plugin_rbac_model::{
    CreateUserRequest, ResetMemberPasswordRequest, RoleItem, UpdateMemberRequest,
    UpdateMemberRolesRequest, UserItem, permission_label,
};
use az_ui_components::{
    admin::{AsyncResult, EditorDialog},
    checkbox::{Checkbox, CheckboxState},
    input::Input,
};
use dioxus::prelude::*;

#[component]
pub(super) fn UserEditor(
    value: Option<UserItem>,
    on_close: Callback<()>,
    on_saved: Callback<()>,
) -> Element {
    let id = value.as_ref().map(|user| user.id.clone());
    let mut account = use_signal(|| {
        value
            .as_ref()
            .map(|u| u.account.clone())
            .unwrap_or_default()
    });
    let mut name = use_signal(|| {
        value
            .as_ref()
            .map(|u| u.display_name.clone())
            .unwrap_or_default()
    });
    let mut password = use_signal(String::new);
    let editing = id.is_some();
    rsx! {
        EditorDialog { title: if editing { "编辑用户" } else { "新建用户" }, description: if editing { "显示名称只在当前租户生效。" } else { "新账号加入当前租户，初始密码至少 12 个字符。" }, on_close, on_saved,
            save: move |_| -> AsyncResult<()> {
                let id = id.clone(); let name = name(); let account = account(); let password = password();
                Box::pin(async move { match id {
                    Some(id) => http::save(&format!("/api/rbac/users/{id}"), &UpdateMemberRequest { display_name: name }, false).await,
                    None => http::save("/api/rbac/users", &CreateUserRequest { account, display_name: name, password }, true).await,
                } })
            },
            label { class: "admin-field", span { "账号" } Input { aria_label: "用户账号", value: account(), disabled: editing, required: true, maxlength: "64", oninput: move |event: FormEvent| account.set(event.value()) } }
            label { class: "admin-field", span { "显示名称" } Input { aria_label: "用户显示名称", value: name(), required: true, maxlength: "96", oninput: move |event: FormEvent| name.set(event.value()) } }
            if !editing { label { class: "admin-field", span { "初始密码" } Input { aria_label: "初始密码", r#type: "password", autocomplete: "new-password", value: password(), required: true, minlength: 12, oninput: move |event: FormEvent| password.set(event.value()) } } }
        }
    }
}

/// 管理员为成员设置新密码；不要求旧密码，成功后该账号所有会话失效。
#[component]
pub(super) fn UserPassword(
    value: UserItem,
    on_close: Callback<()>,
    on_saved: Callback<()>,
) -> Element {
    let mut password = use_signal(String::new);
    let mut confirmation = use_signal(String::new);
    rsx! {
        EditorDialog { title: "重置密码", description: format!("为 {} (@{}) 设置新密码；保存后该账号其他登录将失效。", value.display_name, value.account), on_close, on_saved,
            save: move |_| -> AsyncResult<()> {
                let id = value.id.clone(); let new_password = password(); let confirmation = confirmation();
                Box::pin(async move {
                    if new_password != confirmation { return Err("两次输入的新密码不一致".into()); }
                    http::save(&format!("/api/rbac/users/{id}/password"), &ResetMemberPasswordRequest { new_password }, true).await
                })
            },
            label { class: "admin-field", span { "新密码" } Input { aria_label: "新密码", r#type: "password", autocomplete: "new-password", required: true, minlength: 8, value: password(), oninput: move |event: FormEvent| password.set(event.value()) } }
            label { class: "admin-field", span { "确认新密码" } Input { aria_label: "确认新密码", r#type: "password", autocomplete: "new-password", required: true, minlength: 8, value: confirmation(), oninput: move |event: FormEvent| confirmation.set(event.value()) } }
        }
    }
}

#[component]
pub(super) fn UserRoles(
    value: UserItem,
    roles: Vec<RoleItem>,
    on_close: Callback<()>,
    on_saved: Callback<()>,
) -> Element {
    let mut selected = use_signal(|| value.roles.clone());
    rsx! {
        EditorDialog { title: "分配角色", description: format!("{} (@{})", value.display_name, value.account), on_close, on_saved,
            save: move |_| -> AsyncResult<()> {
                let id = value.id.clone(); let roles = selected();
                Box::pin(async move { http::save(&format!("/api/rbac/users/{id}/roles"), &UpdateMemberRolesRequest { roles }, false).await })
            },
            div { class: "admin-options",
                for role in roles {
                    label { class: "admin-option", key: "{role.id}",
                        Checkbox { aria_label: "角色 {role.id}", checked: Some(if selected().contains(&role.id) { CheckboxState::Checked } else { CheckboxState::Unchecked }),
                            on_checked_change: { let id = role.id.clone(); move |state| { selected.write().retain(|item| item != &id); if bool::from(state) { selected.write().push(id.clone()); } } },
                        }
                        span { span { "{role.id}" } small { class: "admin-meta", "{role.permissions.iter().map(|p| permission_label(p)).collect::<Vec<_>>().join(\"、\")}" } }
                    }
                }
            }
        }
    }
}
