use crate::http;
use aio_plugin_rbac_model::{CreateRoleRequest, RoleItem, permission_label};
use az_ui_components::{
    admin::{AsyncResult, EditorDialog},
    checkbox::{Checkbox, CheckboxState},
    input::Input,
};
use dioxus::prelude::*;

#[component]
pub(super) fn RoleEditor(
    value: Option<RoleItem>,
    permissions: Vec<String>,
    on_close: Callback<()>,
    on_saved: Callback<()>,
) -> Element {
    let editing = value.is_some();
    let mut role_id = use_signal(|| value.as_ref().map(|r| r.id.clone()).unwrap_or_default());
    let mut selected = use_signal(|| {
        value
            .as_ref()
            .map(|r| r.permissions.clone())
            .unwrap_or_default()
    });
    rsx! {
        EditorDialog { title: if editing { "编辑角色" } else { "新建角色" }, description: "角色权限在当前租户生效。", on_close, on_saved,
            save: move |_| -> AsyncResult<()> {
                let body = CreateRoleRequest { role_id: role_id(), permissions: selected() };
                let path = if editing { format!("/api/rbac/roles/{}", body.role_id) } else { "/api/rbac/roles".into() };
                Box::pin(async move { http::save(&path, &body, !editing).await })
            },
            label { class: "admin-field", span { "角色 ID" } Input { aria_label: "角色 ID", value: role_id(), disabled: editing, required: true, maxlength: "64", oninput: move |event: FormEvent| role_id.set(event.value()) } }
            fieldset { class: "admin-options", legend { "权限" }
                for permission in permissions {
                    label { class: "admin-option", key: "{permission}",
                        Checkbox { aria_label: "{permission_label(&permission)}", checked: Some(if selected().contains(&permission) { CheckboxState::Checked } else { CheckboxState::Unchecked }),
                            on_checked_change: { let value = permission.clone(); move |state| { selected.write().retain(|p| p != &value); if bool::from(state) { selected.write().push(value.clone()); } } },
                        }
                        span { "{permission_label(&permission)}" }
                    }
                }
            }
        }
    }
}
