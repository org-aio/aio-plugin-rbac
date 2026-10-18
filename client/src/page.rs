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

/// 从宿主下发的页面 ID 推断当前区块；未知页面回退到用户管理。
fn read_view() -> Option<Section> {
    let location = web_sys::window()?.location();
    let search = location.search().unwrap_or_default();
    let page = search
        .trim_start_matches('?')
        .split('&')
        .find_map(|pair| pair.strip_prefix("page="))
        .and_then(|value| urlencoding::decode(value).ok())
        .map(|value| value.into_owned())
        .unwrap_or_default();
    let view = page.rsplit(':').next().unwrap_or_default();
    Some(match view {
        "roles" => Section::Roles,
        "departments" => Section::Departments,
        "posts" => Section::Posts,
        _ => Section::Users,
    })
}
