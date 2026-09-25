use super::dialogs::DepartmentEditor;
use crate::http;
use aio_plugin_rbac_model::OrganizationItem;
use az_ui_components::{
    admin::{
        AsyncResult, CollectionTable, DeleteRecordsDialog, PageHeader, PageSurface, RequestState,
        SortValue, StatusMessage,
    },
    button::{Button, ButtonSize, ButtonVariant},
    data_table::{DataTableCellContext, DataTableColumn},
};
use dioxus::prelude::*;
use dioxus_icons::lucide::{Pencil, Plus, RefreshCw, Trash2};

#[allow(non_snake_case)]
pub(crate) fn DepartmentsPage() -> Element {
    let mut revision = use_signal(|| 0_u64);
    let access = use_resource(move || {
        let _ = revision();
        http::load()
    });
    let mut editor = use_signal(|| None::<Option<OrganizationItem>>);
    let mut removal = use_signal(|| None::<OrganizationItem>);
    let mut feedback = use_signal(|| None::<String>);
    let data = match access.read().as_ref().cloned() {
        Some(Ok(value)) => value,
        Some(Err(error)) => {
            return rsx! { PageSurface { RequestState { error, on_retry: move |_| revision += 1 } } };
        }
        None => return rsx! { PageSurface { RequestState {} } },
    };
    let departments = data.departments.clone();
    let parent_names = departments
        .iter()
        .map(|department| (department.id.clone(), department.name.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();
    rsx! {
        PageSurface {
            PageHeader { title: "部门管理", detail: format!("当前租户 · {} 个部门", departments.len()),
                Button { size: ButtonSize::Icon, variant: ButtonVariant::Outline, title: "刷新部门", aria_label: "刷新部门", onclick: move |_| revision += 1, RefreshCw {} }
                Button { onclick: move |_| editor.set(Some(None)), Plus {} "新建部门" }
            }
            if let Some(message) = feedback() { StatusMessage { message } }
            CollectionTable { label: "部门", rows: departments,
                columns: vec![DataTableColumn::leaf("name", "部门名称").width(220), DataTableColumn::leaf("parent", "上级部门").width(180), DataTableColumn::leaf("leader", "负责人").width(140), DataTableColumn::leaf("contact", "联系方式").width(220), DataTableColumn::leaf("sort", "排序").width(90), DataTableColumn::leaf("actions", "操作").width(110)],
                row_key: |department: OrganizationItem| department.id, search_text: |department: OrganizationItem| format!("{} {} {} {} {}", department.name, department.leader, department.phone, department.email, department.parent_id.clone().unwrap_or_default()),
                sort_value: |(department, key): (OrganizationItem, String)| if key == "sort" { SortValue::Number(department.sort_order as i128) } else { SortValue::Text(department.name) }, sortable: vec!["name".into(), "sort".into()],
                render_cell: move |context: DataTableCellContext<OrganizationItem>| {
                    let department = context.row;
                    let parent = department.parent_id.as_ref().and_then(|id| parent_names.get(id).cloned()).unwrap_or_else(|| "顶级部门".into());
                    match context.column.key.as_str() {
                        "name" => rsx! { "{department.name}" },
                        "parent" => rsx! { "{parent}" },
                        "leader" => rsx! { if department.leader.is_empty() { "未设置" } else { "{department.leader}" } },
                        "contact" => rsx! { "{department.phone} {department.email}" },
                        "sort" => rsx! { "{department.sort_order}" },
                        "actions" => { let edit = department.clone(); rsx! { div { class: "admin-actions",
                            Button { size: ButtonSize::IconSm, variant: ButtonVariant::Ghost, title: "编辑 {department.name}", aria_label: "编辑 {department.name}", onclick: move |_| editor.set(Some(Some(edit.clone()))), Pencil {} }
                            Button { size: ButtonSize::IconSm, variant: ButtonVariant::Ghost, title: "删除 {department.name}", aria_label: "删除 {department.name}", onclick: move |_| removal.set(Some(department.clone())), Trash2 {} }
                        } } }, _ => rsx! {},
                    }
                },
            }
        }
        if let Some(value) = editor() {
            DepartmentEditor { value, departments: data.departments, on_close: move |_| editor.set(None), on_saved: move |_| { editor.set(None); feedback.set(Some("部门已保存".into())); revision += 1; } }
        }
        if let Some(value) = removal() { DeleteRecordsDialog { title: "删除部门", items: vec![value], item_label: |department: OrganizationItem| department.name,
            delete: |department: OrganizationItem| -> AsyncResult<()> { Box::pin(async move { http::delete(&format!("/api/rbac/departments/{}", department.id)).await }) },
            on_close: move |_| removal.set(None), on_deleted: move |_| { feedback.set(Some("部门已删除".into())); revision += 1; },
        } }
    }
}
