mod http;
mod organization;
mod roles;
mod users;

use az_dioxus_admin_shell::{
    ApplicationMenuGroup, ApplicationPage, ApplicationPlugin, ApplicationScene,
};
use dill::CatalogBuilder;

#[derive(Debug)]
pub struct AccessControlPlugin;

impl ApplicationPlugin for AccessControlPlugin {
    fn pages(&self) -> Vec<ApplicationPage> {
        vec![
            ApplicationPage {
                id: "users",
                label: "用户管理",
                icon: Some("user"),
                scene: system_scene(),
                menu_path: system_management_path(),
                required_permission: Some("rbac:manage"),
                render: users::UsersPage,
            },
            ApplicationPage {
                id: "roles",
                label: "角色管理",
                icon: Some("shield"),
                scene: system_scene(),
                menu_path: system_management_path(),
                required_permission: Some("rbac:manage"),
                render: roles::RolesPage,
            },
            ApplicationPage {
                id: "departments",
                label: "部门管理",
                icon: Some("building_2"),
                scene: system_scene(),
                menu_path: system_management_path(),
                required_permission: Some("rbac:manage"),
                render: organization::DepartmentsPage,
            },
            ApplicationPage {
                id: "posts",
                label: "岗位管理",
                icon: Some("briefcase"),
                scene: system_scene(),
                menu_path: system_management_path(),
                required_permission: Some("rbac:manage"),
                render: organization::PostsPage,
            },
        ]
    }
}

fn system_scene() -> ApplicationScene {
    ApplicationScene {
        id: "system",
        label: "系统",
    }
}

fn system_management_path() -> Vec<ApplicationMenuGroup> {
    vec![ApplicationMenuGroup {
        id: "system-management".to_owned(),
        label: "系统管理".to_owned(),
        icon: Some("settings".to_owned()),
    }]
}

pub fn register(builder: &mut CatalogBuilder) {
    builder
        .add_value(AccessControlPlugin)
        .bind::<dyn ApplicationPlugin, AccessControlPlugin>();
}
