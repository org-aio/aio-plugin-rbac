use az_ui_components::admin::{PageSurface, RequestState};
use dioxus::prelude::*;

/// 整包只有一个挂载点，宿主按页面 ID 传入要展示的管理界面。
#[component]
pub fn AccessControlPage() -> Element {
    let view = use_signal(read_view);
    match view() {
        None => rsx! { PageSurface { RequestState {} } },
        Some(section) => match section {
            Section::Users => rsx! { crate::users::UsersPage {} },
            Section::Roles => rsx! { crate::roles::RolesPage {} },
            Section::Departments => rsx! { crate::organization::DepartmentsPage {} },
            Section::Posts => rsx! { crate::organization::PostsPage {} },
        },
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Section {
    Users,
    Roles,
    Departments,
    Posts,
}

/// 从宿主渲染的页面 ID 推断当前区块；未知页面回退到用户管理。
fn read_view() -> Option<Section> {
    let location = web_sys::window()?.location();
    let hash = location.hash().unwrap_or_default();
    let path = location.pathname().unwrap_or_default();
    let marker = format!("{hash}{path}");
    Some(if marker.contains("roles") {
        Section::Roles
    } else if marker.contains("departments") {
        Section::Departments
    } else if marker.contains("posts") {
        Section::Posts
    } else {
        Section::Users
    })
}
