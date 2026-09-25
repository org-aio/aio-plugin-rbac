use crate::AccessControlService;
use anyhow::{Result, ensure};
use sqlx::postgres::PgPoolOptions;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::test]
#[ignore = "需要本机 aio_keepalive_test 独立测试数据库"]
async fn tenant_role_crud_preserves_authority_and_memberships() -> Result<()> {
    let database = std::env::var("AIO_TEST_DATABASE_URL")?;
    ensure!(
        database.contains("@127.0.0.1:") && database.ends_with("/aio_keepalive_test"),
        "仅允许独立测试库"
    );
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database)
        .await?;
    let service = AccessControlService { pool: pool.clone() };
    let tenant = format!(
        "rbac-test-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    );
    let other = format!("{tenant}-other");
    let a = format!("{tenant}-a");
    let b = format!("{tenant}-b");
    let member = format!("{tenant}-member");
    let outsider = format!("{tenant}-outsider");
    let ids = [&a, &b, &member, &outsider];
    let outcome: Result<()> = async {
        for id in [&tenant, &other] { sqlx::query("INSERT INTO tenants (id, label) VALUES ($1, $1)").bind(id).execute(&pool).await?; }
        for id in ids {
            sqlx::query("INSERT INTO identity_users (id, account, display_name, password_hash) VALUES ($1, $1, $1, 'test-no-login')").bind(id).execute(&pool).await?;
            sqlx::query("INSERT INTO tenant_memberships (tenant_id, user_id, display_name) VALUES ($1, $2, $2)").bind(if id == &outsider { &other } else { &tenant }).bind(id).execute(&pool).await?;
        }
        for id in [&a, &b] {
            sqlx::query("INSERT INTO tenant_member_roles (tenant_id, user_id, role_id) VALUES ($1, $2, 'tenant-admin')").bind(&tenant).bind(id).execute(&pool).await?;
        }
        service.initialize().await?;
        service.save_role(&tenant, &a, "editor", &["file:manage".into()], true).await?;
        assert!(service.save_role(&tenant, &a, "editor", &["file:manage".into()], true).await.is_err());
        service.set_member_roles(&tenant, &a, &member, &["member".into(), "editor".into()], false).await?;
        assert!(service.delete_role(&tenant, &a, "editor").await.is_err());
        service.update_member(&tenant, &a, &member, "新的租户昵称").await?;
        let view = service.view(&tenant, &a).await?;
        assert_eq!(view.users.iter().find(|u| u.id == member).unwrap().display_name, "新的租户昵称");
        assert_eq!(view.roles.iter().find(|r| r.id == "editor").unwrap().member_count, 1);
        assert!(view.users.iter().all(|u| u.id != outsider));
        assert!(service.update_member(&tenant, &a, &outsider, "越权").await.is_err());
        assert!(service.set_member_roles(&tenant, &a, &outsider, &["editor".into()], false).await.is_err());
        assert!(service.save_role(&tenant, &a, "tenant-admin", &["workspace:view".into()], false).await.is_err());
        assert!(service.delete_role(&tenant, &a, "member").await.is_err());
        assert!(service.remove_member(&tenant, &a, &a).await.is_err());
        assert!(service.set_member_roles(&tenant, &a, &a, &["member".into()], false).await.is_err());

        service.save_role(&tenant, &a, "limited-manager", &["rbac:manage".into()], true).await?;
        service.set_member_roles(&tenant, &a, &member, &["limited-manager".into()], false).await?;
        assert!(service.save_role(&tenant, &member, "escalation", &["file:manage".into()], true).await.is_err());
        assert!(service.set_member_roles(&tenant, &member, &member, &["tenant-admin".into()], false).await.is_err());
        assert!(service.remove_member(&tenant, &member, &a).await.is_err());
        service.set_member_roles(&tenant, &a, &member, &["member".into()], false).await?;
        assert!(service.save_role(&tenant, &member, "denied", &["workspace:view".into()], true).await.is_err());
        service.delete_role(&tenant, &a, "editor").await?;
        service.delete_role(&tenant, &a, "limited-manager").await?;
        service.save_department(&tenant, &a, aio_plugin_rbac_model::SaveDepartmentRequest {
            id: None,
            parent_id: None,
            name: "工程部".into(),
            leader: "负责人".into(),
            phone: "10086".into(),
            email: "dept@example.com".into(),
            sort_order: 1,
            status: "active".into(),
        }).await?;
        let department = service.departments(&tenant).await?.into_iter().next().expect("部门应存在");
        assert_eq!(department.name, "工程部");
        assert!(service.departments(&other).await?.is_empty());
        service.save_post(&tenant, &a, aio_plugin_rbac_model::SavePostRequest {
            id: None,
            code: "site-engineer".into(),
            name: "现场工程师".into(),
            sort_order: 1,
            status: "active".into(),
            remark: "测试岗位".into(),
        }).await?;
        let post = service.posts(&tenant).await?.into_iter().next().expect("岗位应存在");
        assert_eq!(post.code, "site-engineer");
        assert!(service.posts(&other).await?.is_empty());
        service.delete_post(&tenant, &a, &post.id).await?;
        service.delete_department(&tenant, &a, &department.id).await?;
        sqlx::query("INSERT INTO auth_sessions (id, user_id, tenant_id, expires_at) VALUES ($1, $2, $3, now() + interval '1 hour')")
            .bind(&member).bind(&member).bind(&tenant).execute(&pool).await?;
        service.remove_member(&tenant, &a, &member).await?;
        assert!(!sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM auth_sessions WHERE user_id = $1)").bind(&member).fetch_one(&pool).await?);
        assert!(sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM identity_users WHERE id = $1)").bind(&member).fetch_one(&pool).await?);

        let (one, two) = tokio::join!(service.remove_member(&tenant, &a, &b), service.remove_member(&tenant, &b, &a));
        assert_ne!(one.is_ok(), two.is_ok(), "并发撤销不能移除全部管理员");
        assert_eq!(service.view(&tenant, &a).await?.users.len(), 1);
        Ok(())
    }.await;
    for table in [
        "auth_sessions",
        "tenant_member_roles",
        "organization_departments",
        "organization_posts",
        "tenant_memberships",
        "role_permissions",
    ] {
        sqlx::query(&format!("DELETE FROM {table} WHERE tenant_id IN ($1, $2)"))
            .bind(&tenant)
            .bind(&other)
            .execute(&pool)
            .await?;
    }
    for id in ids {
        sqlx::query("DELETE FROM identity_users WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await?;
    }
    sqlx::query("DELETE FROM tenants WHERE id IN ($1, $2)")
        .bind(&tenant)
        .bind(&other)
        .execute(&pool)
        .await?;
    outcome
}
