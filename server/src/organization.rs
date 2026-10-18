use crate::AccessControlService;
use aio_plugin_rbac_model::{OrganizationItem, SaveDepartmentRequest, SavePostRequest};
use anyhow::{Result, ensure};
use sqlx::Row;

fn department(row: &sqlx::postgres::PgRow) -> Result<OrganizationItem> {
    Ok(OrganizationItem {
        id: row.try_get("id")?,
        code: String::new(),
        name: row.try_get("name")?,
        parent_id: row.try_get("parent_id")?,
        leader: row.try_get("leader")?,
        phone: row.try_get("phone")?,
        email: row.try_get("email")?,
        sort_order: row.try_get("sort_order")?,
        status: row.try_get("status")?,
        remark: String::new(),
    })
}

fn post(row: &sqlx::postgres::PgRow) -> Result<OrganizationItem> {
    Ok(OrganizationItem {
        id: row.try_get("id")?,
        code: row.try_get("code")?,
        name: row.try_get("name")?,
        parent_id: None,
        leader: String::new(),
        phone: String::new(),
        email: String::new(),
        sort_order: row.try_get("sort_order")?,
        status: row.try_get("status")?,
        remark: row.try_get("remark")?,
    })
}

fn validate_status(status: &str) -> Result<()> {
    ensure!(matches!(status, "active" | "disabled"), "状态无效");
    Ok(())
}

impl AccessControlService {
    pub async fn departments(&self, tenant: &str) -> Result<Vec<OrganizationItem>> {
        let rows = sqlx::query(
            "SELECT id,name,parent_id,leader,phone,email,sort_order,status FROM organization_departments WHERE tenant_id=$1 ORDER BY sort_order,id",
        )
        .bind(tenant)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(department).collect()
    }

    pub async fn save_department(
        &self,
        tenant: &str,
        actor: &str,
        request: SaveDepartmentRequest,
    ) -> Result<()> {
        let name = request.name.trim();
        ensure!(
            !name.is_empty() && name.chars().count() <= 120,
            "部门名称需要 1 到 120 个字符"
        );
        validate_status(request.status.trim())?;
        let id = request
            .id
            .unwrap_or_else(|| format!("dept-{}", uuid::Uuid::new_v4().simple()));
        let parent_id = request.parent_id.filter(|value| !value.trim().is_empty());
        if let Some(parent) = parent_id.as_deref() {
            ensure!(parent != id, "部门不能把自己设为上级");
            let parent_exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM organization_departments WHERE tenant_id=$1 AND id=$2)",
            )
            .bind(tenant)
            .bind(parent)
            .fetch_one(&self.pool)
            .await?;
            ensure!(parent_exists, "上级部门不存在或不属于当前租户");
        }
        let mut tx = self.mutation(tenant, actor).await?;
        sqlx::query(
            "INSERT INTO organization_departments(tenant_id,id,parent_id,name,leader,phone,email,sort_order,status) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT(tenant_id,id) DO UPDATE SET parent_id=EXCLUDED.parent_id,name=EXCLUDED.name,leader=EXCLUDED.leader,phone=EXCLUDED.phone,email=EXCLUDED.email,sort_order=EXCLUDED.sort_order,status=EXCLUDED.status,updated_at=now()",
        )
        .bind(tenant)
        .bind(id)
        .bind(parent_id)
        .bind(name)
        .bind(request.leader.trim())
        .bind(request.phone.trim())
        .bind(request.email.trim())
        .bind(request.sort_order)
        .bind(request.status.trim())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn delete_department(&self, tenant: &str, actor: &str, id: &str) -> Result<()> {
        let mut tx = self.mutation(tenant, actor).await?;
        let has_children = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM organization_departments WHERE tenant_id=$1 AND parent_id=$2)",
        )
        .bind(tenant)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        ensure!(!has_children, "请先删除下级部门");
        let deleted =
            sqlx::query("DELETE FROM organization_departments WHERE tenant_id=$1 AND id=$2")
                .bind(tenant)
                .bind(id)
                .execute(&mut *tx)
                .await?
                .rows_affected();
        ensure!(deleted == 1, "部门不存在或不属于当前租户");
        tx.commit().await?;
        Ok(())
    }

    pub async fn posts(&self, tenant: &str) -> Result<Vec<OrganizationItem>> {
        let rows = sqlx::query(
            "SELECT id,code,name,sort_order,status,remark FROM organization_posts WHERE tenant_id=$1 ORDER BY sort_order,id",
        )
        .bind(tenant)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(post).collect()
    }

    pub async fn save_post(
        &self,
        tenant: &str,
        actor: &str,
        request: SavePostRequest,
    ) -> Result<()> {
        let name = request.name.trim();
        let code = request.code.trim();
        ensure!(
            !name.is_empty() && name.chars().count() <= 120,
            "岗位名称需要 1 到 120 个字符"
        );
        ensure!(
            !code.is_empty()
                && code.len() <= 64
                && code
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')),
            "岗位编码必须为 1 到 64 位字母、数字、连字符或下划线"
        );
        validate_status(request.status.trim())?;
        let id = request
            .id
            .unwrap_or_else(|| format!("post-{}", uuid::Uuid::new_v4().simple()));
        let mut tx = self.mutation(tenant, actor).await?;
        sqlx::query(
            "INSERT INTO organization_posts(tenant_id,id,code,name,sort_order,status,remark) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(tenant_id,id) DO UPDATE SET code=EXCLUDED.code,name=EXCLUDED.name,sort_order=EXCLUDED.sort_order,status=EXCLUDED.status,remark=EXCLUDED.remark,updated_at=now()",
        )
        .bind(tenant)
        .bind(id)
        .bind(code)
        .bind(name)
        .bind(request.sort_order)
        .bind(request.status.trim())
        .bind(request.remark.trim())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn delete_post(&self, tenant: &str, actor: &str, id: &str) -> Result<()> {
        let mut tx = self.mutation(tenant, actor).await?;
        let deleted = sqlx::query("DELETE FROM organization_posts WHERE tenant_id=$1 AND id=$2")
            .bind(tenant)
            .bind(id)
            .execute(&mut *tx)
            .await?
            .rows_affected();
        ensure!(deleted == 1, "岗位不存在或不属于当前租户");
        tx.commit().await?;
        Ok(())
    }
}
