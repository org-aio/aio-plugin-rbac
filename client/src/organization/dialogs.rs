use crate::http;
use aio_plugin_rbac_model::{OrganizationItem, SaveDepartmentRequest, SavePostRequest};
use az_ui_components::{
    admin::{AsyncResult, EditorDialog},
    input::Input,
    textarea::Textarea,
};
use dioxus::prelude::*;

#[component]
pub(super) fn DepartmentEditor(
    value: Option<OrganizationItem>,
    departments: Vec<OrganizationItem>,
    on_close: Callback<()>,
    on_saved: Callback<()>,
) -> Element {
    let id = value.as_ref().map(|item| item.id.clone());
    let current_id = id.clone();
    let mut name = use_signal(|| {
        value
            .as_ref()
            .map(|item| item.name.clone())
            .unwrap_or_default()
    });
    let mut leader = use_signal(|| {
        value
            .as_ref()
            .map(|item| item.leader.clone())
            .unwrap_or_default()
    });
    let mut phone = use_signal(|| {
        value
            .as_ref()
            .map(|item| item.phone.clone())
            .unwrap_or_default()
    });
    let mut email = use_signal(|| {
        value
            .as_ref()
            .map(|item| item.email.clone())
            .unwrap_or_default()
    });
    let mut parent_id = use_signal(|| {
        value
            .as_ref()
            .and_then(|item| item.parent_id.clone())
            .unwrap_or_default()
    });
    let mut sort_order = use_signal(|| value.as_ref().map(|item| item.sort_order).unwrap_or(0));
    let editing = id.is_some();
    rsx! {
        EditorDialog {
            title: if editing { "编辑部门" } else { "新建部门" },
            description: "部门在当前租户内共享，子部门不能形成环路。",
            on_close,
            on_saved,
            save: move |_| -> AsyncResult<()> {
                let body = SaveDepartmentRequest {
                    id: id.clone(),
                    parent_id: (!parent_id().is_empty()).then(&*parent_id),
                    name: name(),
                    leader: leader(),
                    phone: phone(),
                    email: email(),
                    sort_order: sort_order(),
                    status: "active".into(),
                };
                let path = id.as_ref().map_or_else(|| "/api/rbac/departments".into(), |id| format!("/api/rbac/departments/{id}"));
                Box::pin(async move { http::save(&path, &body, !editing).await })
            },
            label { class: "admin-field", span { "部门名称" } Input { aria_label: "部门名称", value: name(), required: true, maxlength: "120", oninput: move |event: FormEvent| name.set(event.value()) } }
            label { class: "admin-field", span { "上级部门" }
                select { class: "dx-input", value: parent_id(), onchange: move |event: FormEvent| parent_id.set(event.value()) ,
                    option { value: "", "无上级部门" }
                    for department in departments.into_iter().filter(|department| Some(&department.id) != current_id.as_ref()) {
                        option { value: "{department.id}", "{department.name}" }
                    }
                }
            }
            label { class: "admin-field", span { "负责人" } Input { aria_label: "部门负责人", value: leader(), oninput: move |event: FormEvent| leader.set(event.value()) } }
            label { class: "admin-field", span { "联系电话" } Input { aria_label: "部门联系电话", value: phone(), oninput: move |event: FormEvent| phone.set(event.value()) } }
            label { class: "admin-field", span { "邮箱" } Input { aria_label: "部门邮箱", value: email(), oninput: move |event: FormEvent| email.set(event.value()) } }
            label { class: "admin-field", span { "排序" } Input { aria_label: "部门排序", r#type: "number", value: sort_order().to_string(), oninput: move |event: FormEvent| sort_order.set(event.value().parse().unwrap_or(0)) } }
        }
    }
}

#[component]
pub(super) fn PostEditor(
    value: Option<OrganizationItem>,
    on_close: Callback<()>,
    on_saved: Callback<()>,
) -> Element {
    let id = value.as_ref().map(|item| item.id.clone());
    let mut code = use_signal(|| {
        value
            .as_ref()
            .map(|item| item.code.clone())
            .unwrap_or_default()
    });
    let mut name = use_signal(|| {
        value
            .as_ref()
            .map(|item| item.name.clone())
            .unwrap_or_default()
    });
    let mut sort_order = use_signal(|| value.as_ref().map(|item| item.sort_order).unwrap_or(0));
    let mut remark = use_signal(|| {
        value
            .as_ref()
            .map(|item| item.remark.clone())
            .unwrap_or_default()
    });
    let editing = id.is_some();
    rsx! {
        EditorDialog {
            title: if editing { "编辑岗位" } else { "新建岗位" },
            description: "岗位编码在当前租户内唯一。",
            on_close,
            on_saved,
            save: move |_| -> AsyncResult<()> {
                let body = SavePostRequest { id: id.clone(), code: code(), name: name(), sort_order: sort_order(), status: "active".into(), remark: remark() };
                let path = id.as_ref().map_or_else(|| "/api/rbac/posts".into(), |id| format!("/api/rbac/posts/{id}"));
                Box::pin(async move { http::save(&path, &body, !editing).await })
            },
            label { class: "admin-field", span { "岗位编码" } Input { aria_label: "岗位编码", value: code(), disabled: editing, required: true, maxlength: "64", oninput: move |event: FormEvent| code.set(event.value()) } }
            label { class: "admin-field", span { "岗位名称" } Input { aria_label: "岗位名称", value: name(), required: true, maxlength: "120", oninput: move |event: FormEvent| name.set(event.value()) } }
            label { class: "admin-field", span { "排序" } Input { aria_label: "岗位排序", r#type: "number", value: sort_order().to_string(), oninput: move |event: FormEvent| sort_order.set(event.value().parse().unwrap_or(0)) } }
            label { class: "admin-field", span { "备注" } Textarea { aria_label: "岗位备注", value: remark(), oninput: move |event: FormEvent| remark.set(event.value()) } }
        }
    }
}
