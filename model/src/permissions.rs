/// Display names only; authorization and submitted values always use the original code.
pub fn permission_label(permission: &str) -> &str {
    let name = permission
        .strip_prefix("component:")
        .and_then(|scoped| scoped.split_once(':'))
        .map_or(permission, |(_, name)| name);
    match name {
        "workspace:view" => "访问工作区",
        "rbac:manage" => "管理用户与角色",
        "tenant:manage" => "管理租户",
        "plugin:manage" => "管理插件",
        "dictionary:manage" => "管理字典",
        "file:manage" => "管理文件",
        "user:manage" => "管理用户",
        "screen.view" => "查看大屏",
        "screen.edit" => "编辑大屏",
        "screen.data" => "管理大屏数据",
        "screen.publish" => "发布大屏",
        "memory:compile" => "编译记忆",
        _ => "未命名权限",
    }
}

#[cfg(test)]
mod tests {
    use super::permission_label;

    #[test]
    fn component_permissions_use_the_same_labels_as_unscoped_permissions() {
        for (code, label) in [
            ("screen.view", "查看大屏"),
            ("screen.edit", "编辑大屏"),
            ("screen.data", "管理大屏数据"),
            ("screen.publish", "发布大屏"),
            ("memory:compile", "编译记忆"),
        ] {
            assert_eq!(permission_label(code), label);
            assert_eq!(
                permission_label(&format!(
                    "component:68bbdcee-8522-4574-b97d-0530d45fca7e:{code}"
                )),
                label
            );
        }
    }

    #[test]
    fn built_in_and_unknown_permissions_never_expose_codes() {
        assert_eq!(permission_label("rbac:manage"), "管理用户与角色");
        for code in [
            "unknown:action",
            "component:id:unknown:action",
            "component:broken",
        ] {
            assert_eq!(permission_label(code), "未命名权限");
        }
    }
}
