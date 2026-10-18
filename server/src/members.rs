use crate::{
    AccessControlService,
    policy::{ensure_grantable, ensure_managers, role_permissions},
};
use anyhow::{Result, ensure};
use sqlx::{Postgres, Transaction};

async fn member_roles(
    tx: &mut Transaction<'_, Postgres>,
    tenant: &str,
    user: &str,
) -> Result<Vec<String>> {
    let member = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM tenant_memberships WHERE tenant_id = $1 AND user_id = $2)",
    )
    .bind(tenant)
    .bind(user)
    .fetch_one(&mut **tx)
    .await?;
    ensure!(member, "用户不存在或不属于当前租户");
    Ok(sqlx::query_scalar(
        "SELECT role_id FROM tenant_member_roles WHERE tenant_id = $1 AND user_id = $2",
    )
    .bind(tenant)
    .bind(user)
    .fetch_all(&mut **tx)
    .await?)
}

async fn check_roles(
    tx: &mut Transaction<'_, Postgres>,
    tenant: &str,
    actor: &str,
    roles: &[String],
) -> Result<()> {
    for role in roles {
        let permissions = role_permissions(tx, tenant, role).await?;
        ensure_grantable(tx, tenant, actor, &permissions).await?;
        if role == "platform-admin" {
            let platform = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM tenant_member_roles WHERE tenant_id = $1 AND user_id = $2 AND role_id = 'platform-admin')")
                .bind(tenant).bind(actor).fetch_one(&mut **tx).await?;
            ensure!(platform, "只有平台管理员可以管理平台管理员角色");
        }
    }
    Ok(())
}

impl AccessControlService {
    pub async fn update_member(
        &self,
        tenant: &str,
        actor: &str,
        user: &str,
        name: &str,
    ) -> Result<()> {
        let name = name.trim();
        ensure!(
            !name.is_empty() && name.chars().count() <= 96,
            "显示名称需要 1 到 96 个字符"
        );
        let mut tx = self.mutation(tenant, actor).await?;
        let roles = member_roles(&mut tx, tenant, user).await?;
        check_roles(&mut tx, tenant, actor, &roles).await?;
        sqlx::query(
            "UPDATE tenant_memberships SET display_name = $3 WHERE tenant_id = $1 AND user_id = $2",
        )
        .bind(tenant)
        .bind(user)
        .bind(name)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// 校验当前管理员是否有权为该成员重置密码。
    /// 只做权限判定，不写库；实际的密码写入由身份服务完成。
    pub async fn authorize_member_password_reset(
        &self,
        tenant: &str,
        actor: &str,
        user: &str,
    ) -> Result<()> {
        let mut tx = self.mutation(tenant, actor).await?;
        let roles = member_roles(&mut tx, tenant, user).await?;
        check_roles(&mut tx, tenant, actor, &roles).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn set_member_roles(
        &self,
        tenant: &str,
        actor: &str,
        user: &str,
        roles: &[String],
        append: bool,
    ) -> Result<()> {
        ensure!(roles.len() <= 128, "最多分配 128 个角色");
        let mut tx = self.mutation(tenant, actor).await?;
        let old = member_roles(&mut tx, tenant, user).await?;
        check_roles(&mut tx, tenant, actor, &old).await?;
        check_roles(&mut tx, tenant, actor, roles).await?;
        if !append {
            sqlx::query("DELETE FROM tenant_member_roles WHERE tenant_id = $1 AND user_id = $2")
                .bind(tenant)
                .bind(user)
                .execute(&mut *tx)
                .await?;
        }
        for role in roles {
            sqlx::query("INSERT INTO tenant_member_roles (tenant_id, user_id, role_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING")
                .bind(tenant).bind(user).bind(role).execute(&mut *tx).await?;
        }
        ensure_managers(&mut tx, tenant, actor).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn remove_member(&self, tenant: &str, actor: &str, user: &str) -> Result<()> {
        ensure!(actor != user, "不能将自己移出当前租户");
        let mut tx = self.mutation(tenant, actor).await?;
        let roles = member_roles(&mut tx, tenant, user).await?;
        check_roles(&mut tx, tenant, actor, &roles).await?;
        for query in [
            "DELETE FROM tenant_member_roles WHERE tenant_id = $1 AND user_id = $2",
            "DELETE FROM auth_sessions WHERE tenant_id = $1 AND user_id = $2",
            "DELETE FROM tenant_memberships WHERE tenant_id = $1 AND user_id = $2",
        ] {
            sqlx::query(query)
                .bind(tenant)
                .bind(user)
                .execute(&mut *tx)
                .await?;
        }
        ensure_managers(&mut tx, tenant, actor).await?;
        tx.commit().await?;
        Ok(())
    }
}
