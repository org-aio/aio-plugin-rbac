use std::{collections::BTreeMap, env};

use aio_plugin_rbac_model::{AccessControlView, RoleItem, UserItem};
use anyhow::{Context as _, Result};
use sqlx::{PgPool, postgres::PgPoolOptions};

#[derive(Debug)]
pub struct AccessControlService {
    pub(crate) pool: PgPool,
}

impl AccessControlService {
    pub fn from_env() -> Result<Self> {
        let database_url = env::var("AIO_DATABASE_URL")
            .or_else(|_| env::var("AZ_AIO_DATABASE_URL"))
            .context("权限插件缺少 AIO_DATABASE_URL")?;
        Ok(Self {
            pool: PgPoolOptions::new()
                .max_connections(8)
                .connect_lazy(&database_url)?,
        })
    }

    pub async fn initialize(&self) -> Result<()> {
        sqlx::raw_sql(include_str!("schema.sql"))
            .execute(&self.pool)
            .await?;
        sqlx::query("INSERT INTO role_permissions (tenant_id, role_id, permission) SELECT id, 'member', 'workspace:view' FROM tenants ON CONFLICT DO NOTHING")
            .execute(&self.pool).await?;
        for permission in [
            "plugin:manage",
            "tenant:manage",
            "rbac:manage",
            "dictionary:manage",
            "file:manage",
        ] {
            sqlx::query("INSERT INTO role_permissions (tenant_id, role_id, permission) SELECT DISTINCT tenant_id, role_id, $1 FROM tenant_member_roles WHERE role_id IN ('platform-admin', 'tenant-admin') ON CONFLICT DO NOTHING")
                .bind(permission).execute(&self.pool).await?;
        }
        Ok(())
    }

    pub async fn view(&self, tenant_id: &str, actor_id: &str) -> Result<AccessControlView> {
        let rows = sqlx::query_as::<_, (String, String, String, Vec<String>)>(
            "SELECT u.id, u.account, m.display_name, COALESCE(array_agg(r.role_id ORDER BY r.role_id) FILTER (WHERE r.role_id IS NOT NULL), ARRAY[]::text[]) FROM tenant_memberships m JOIN identity_users u ON u.id = m.user_id LEFT JOIN tenant_member_roles r ON r.tenant_id = m.tenant_id AND r.user_id = m.user_id WHERE m.tenant_id = $1 GROUP BY u.id, u.account, m.display_name ORDER BY u.account")
            .bind(tenant_id).fetch_all(&self.pool).await?;
        let users = rows
            .into_iter()
            .map(|(id, account, display_name, roles)| UserItem {
                id,
                account,
                display_name,
                roles,
            })
            .collect::<Vec<_>>();
        let permission_rows = sqlx::query_as::<_, (String, String)>("SELECT role_id, permission FROM role_permissions WHERE tenant_id = $1 ORDER BY role_id, permission")
            .bind(tenant_id).fetch_all(&self.pool).await?;
        let mut permissions = BTreeMap::<String, Vec<String>>::new();
        for (role, permission) in permission_rows {
            let values = permissions.entry(role).or_default();
            if !permission.starts_with("component:") {
                values.push(permission);
            }
        }
        let roles = permissions
            .into_iter()
            .map(|(id, permissions)| RoleItem {
                member_count: users.iter().filter(|user| user.roles.contains(&id)).count(),
                id,
                permissions,
            })
            .collect::<Vec<_>>();
        let mut grantable_permissions = vec!["workspace:view".to_owned()];
        if let Some(actor) = users.iter().find(|user| user.id == actor_id) {
            for role in &roles {
                if actor.roles.contains(&role.id) {
                    grantable_permissions.extend(role.permissions.clone());
                }
            }
        }
        grantable_permissions.sort();
        grantable_permissions.dedup();
        Ok(AccessControlView {
            users,
            roles,
            current_user_id: actor_id.to_owned(),
            grantable_permissions,
        })
    }
}
