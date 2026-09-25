use super::dialogs::PostEditor;
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
pub(crate) fn PostsPage() -> Element {
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
    rsx! {
        PageSurface {
            PageHeader { title: "岗位管理", detail: format!("当前租户 · {} 个岗位", data.posts.len()),
                Button { size: ButtonSize::Icon, variant: ButtonVariant::Outline, title: "刷新岗位", aria_label: "刷新岗位", onclick: move |_| revision += 1, RefreshCw {} }
                Button { onclick: move |_| editor.set(Some(None)), Plus {} "新建岗位" }
            }
            if let Some(message) = feedback() { StatusMessage { message } }
            CollectionTable { label: "岗位", rows: data.posts,
                columns: vec![DataTableColumn::leaf("code", "岗位编码").width(180), DataTableColumn::leaf("name", "岗位名称").width(220), DataTableColumn::leaf("sort", "排序").width(90), DataTableColumn::leaf("remark", "备注").width(360), DataTableColumn::leaf("actions", "操作").width(110)],
                row_key: |item: OrganizationItem| item.id, search_text: |item: OrganizationItem| format!("{} {} {}", item.code, item.name, item.remark),
                sort_value: |(item, key): (OrganizationItem, String)| if key == "sort" { SortValue::Number(item.sort_order as i128) } else { SortValue::Text(item.name) }, sortable: vec!["name".into(), "sort".into()],
                render_cell: move |context: DataTableCellContext<OrganizationItem>| {
                    let item = context.row;
                    match context.column.key.as_str() {
                        "code" => rsx! { code { class: "admin-code", "{item.code}" } },
                        "name" => rsx! { "{item.name}" },
                        "sort" => rsx! { "{item.sort_order}" },
                        "remark" => rsx! { if item.remark.is_empty() { "—" } else { "{item.remark}" } },
                        "actions" => { let edit = item.clone(); rsx! { div { class: "admin-actions",
                            Button { size: ButtonSize::IconSm, variant: ButtonVariant::Ghost, title: "编辑 {item.name}", aria_label: "编辑 {item.name}", onclick: move |_| editor.set(Some(Some(edit.clone()))), Pencil {} }
                            Button { size: ButtonSize::IconSm, variant: ButtonVariant::Ghost, title: "删除 {item.name}", aria_label: "删除 {item.name}", onclick: move |_| removal.set(Some(item.clone())), Trash2 {} }
                        } } }, _ => rsx! {},
                    }
                },
            }
        }
        if let Some(value) = editor() {
            PostEditor { value, on_close: move |_| editor.set(None), on_saved: move |_| { editor.set(None); feedback.set(Some("岗位已保存".into())); revision += 1; } }
        }
        if let Some(value) = removal() { DeleteRecordsDialog { title: "删除岗位", items: vec![value], item_label: |item: OrganizationItem| item.name,
            delete: |item: OrganizationItem| -> AsyncResult<()> { Box::pin(async move { http::delete(&format!("/api/rbac/posts/{}", item.id)).await }) },
            on_close: move |_| removal.set(None), on_deleted: move |_| { feedback.set(Some("岗位已删除".into())); revision += 1; },
        } }
    }
}
