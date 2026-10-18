use crate::AccessControlService;
use anyhow::{Result, ensure};
use sqlx::{Postgres, Transaction};

impl AccessControlService {
    // Serialize tenant access mutations, then re-check the actor under the same lock.
    pub(crate) async fn mutation(
        &self,
        tenant: &str,
        actor: &str,
    ) -> Result<Transaction<'_, Postgres>> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET LOCAL statement_timeout = '10s'")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 9182))")
            .bind(tenant)
            .execute(&mut *tx)
            .await?;
        let valid = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM tenant_memberships m JOIN tenant_member_roles r ON r.tenant_id = m.tenant_id AND r.user_id = m.user_id JOIN role_permissions p ON p.tenant_id = r.tenant_id AND p.role_id = r.role_id WHERE m.tenant_id = $1 AND m.user_id = $2 AND p.permission = 'rbac:manage')")
            .bind(tenant).bind(actor).fetch_one(&mut *tx).await?;
        ensure!(valid, "当前用户不再具有权限管理权限");
        Ok(tx)
    }
}

pub(crate) async fn ensure_grantable(
    tx: &mut Transaction<'_, Postgres>,
    tenant: &str,
    actor: &str,
    permissions: &[String],
) -> Result<()> {
    let granted = sqlx::query_scalar::<_, String>("SELECT DISTINCT p.permission FROM tenant_member_roles r JOIN role_permissions p ON p.tenant_id = r.tenant_id AND p.role_id = r.role_id WHERE r.tenant_id = $1 AND r.user_id = $2")
        .bind(tenant).bind(actor).fetch_all(&mut **tx).await?;
    ensure!(
        permissions
            .iter()
            .all(|permission| permission == "workspace:view" || granted.contains(permission)),
        "不能管理超出自身授权范围的权限"
    );
    Ok(())
}

pub(crate) async fn role_permissions(
    tx: &mut Transaction<'_, Postgres>,
    tenant: &str,
    role: &str,
) -> Result<Vec<String>> {
    let permissions = sqlx::query_scalar::<_, String>(
        "SELECT permission FROM role_permissions WHERE tenant_id = $1 AND role_id = $2",
    )
    .bind(tenant)
    .bind(role)
    .fetch_all(&mut **tx)
    .await?;
    ensure!(!permissions.is_empty(), "角色不存在或不属于当前租户");
    Ok(permissions)
}

pub(crate) async fn ensure_managers(
    tx: &mut Transaction<'_, Postgres>,
    tenant: &str,
    actor: &str,
) -> Result<()> {
    let managers = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT r.user_id FROM tenant_member_roles r JOIN tenant_memberships m ON m.tenant_id = r.tenant_id AND m.user_id = r.user_id JOIN role_permissions p ON p.tenant_id = r.tenant_id AND p.role_id = r.role_id WHERE r.tenant_id = $1 AND p.permission = 'rbac:manage'")
        .bind(tenant).fetch_all(&mut **tx).await?;
    ensure!(!managers.is_empty(), "不能移除当前租户最后一个权限管理员");
    ensure!(
        managers.iter().any(|id| id == actor),
        "不能撤销自己的权限管理权限，请由其他管理员操作"
    );
    Ok(())
}
