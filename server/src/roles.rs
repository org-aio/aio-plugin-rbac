use crate::{
    AccessControlService,
    policy::{ensure_grantable, ensure_managers, role_permissions},
};
use aio_plugin_rbac_model::is_system_role;
use anyhow::{Result, ensure};

fn validate(role: &str, permissions: &[String]) -> Result<Vec<String>> {
    ensure!(
        !role.is_empty()
            && role.len() <= 64
            && role
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')),
        "角色 ID 必须为 1 到 64 位字母、数字、连字符或下划线"
    );
    ensure!(
        !permissions.is_empty() && permissions.len() <= 128,
        "角色需要 1 到 128 项权限"
    );
    ensure!(
        permissions
            .iter()
            .all(|permission| !permission.starts_with("component:")),
        "插件权限由租户安装自动获得，无需分配给角色"
    );
    let mut permissions = permissions.to_vec();
    ensure!(
        permissions.iter().all(|p| p.len() <= 128
            && p.split_once(':')
                .is_some_and(|(a, b)| !a.is_empty() && !b.is_empty())
            && p.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '-' | '_' | '.'))),
        "权限必须使用 domain:action 格式"
    );
    permissions.sort();
    permissions.dedup();
    Ok(permissions)
}

impl AccessControlService {
    pub async fn save_role(
        &self,
        tenant: &str,
        actor: &str,
        role: &str,
        permissions: &[String],
        create: bool,
    ) -> Result<()> {
        let permissions = validate(role, permissions)?;
        ensure!(!is_system_role(role), "内置角色不能修改或覆盖");
        let mut tx = self.mutation(tenant, actor).await?;
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM role_permissions WHERE tenant_id = $1 AND role_id = $2)",
        )
        .bind(tenant)
        .bind(role)
        .fetch_one(&mut *tx)
        .await?;
        ensure!(
            exists != create,
            if create {
                "角色已存在"
            } else {
                "角色不存在或不属于当前租户"
            }
        );
        if exists {
            let old = role_permissions(&mut tx, tenant, role).await?;
            ensure_grantable(&mut tx, tenant, actor, &old).await?;
        }
        ensure_grantable(&mut tx, tenant, actor, &permissions).await?;
        sqlx::query("DELETE FROM role_permissions WHERE tenant_id = $1 AND role_id = $2")
            .bind(tenant)
            .bind(role)
            .execute(&mut *tx)
            .await?;
        for permission in permissions {
            sqlx::query(
                "INSERT INTO role_permissions (tenant_id, role_id, permission) VALUES ($1, $2, $3)",
            )
            .bind(tenant)
            .bind(role)
            .bind(permission)
            .execute(&mut *tx)
            .await?;
        }
        ensure_managers(&mut tx, tenant, actor).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn delete_role(&self, tenant: &str, actor: &str, role: &str) -> Result<()> {
        ensure!(!is_system_role(role), "内置角色不能删除");
        let mut tx = self.mutation(tenant, actor).await?;
        let permissions = role_permissions(&mut tx, tenant, role).await?;
        ensure_grantable(&mut tx, tenant, actor, &permissions).await?;
        let used = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM tenant_member_roles WHERE tenant_id = $1 AND role_id = $2)")
            .bind(tenant).bind(role).fetch_one(&mut *tx).await?;
        ensure!(!used, "角色仍分配给用户，请先撤销该角色分配");
        sqlx::query("DELETE FROM role_permissions WHERE tenant_id = $1 AND role_id = $2")
            .bind(tenant)
            .bind(role)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_roles_and_deduplicates_permissions() {
        assert_eq!(
            validate("editor", &["file:manage".into(), "file:manage".into()])
                .unwrap()
                .len(),
            1
        );
        assert!(validate("bad/id", &["file:manage".into()]).is_err());
        assert!(validate("editor", &["file:".into()]).is_err());
        assert!(validate("editor", &[]).is_err());
        assert!(validate("editor", &["component:source:screen.edit".into()]).is_err());
    }
}
