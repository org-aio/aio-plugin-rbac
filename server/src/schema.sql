CREATE TABLE IF NOT EXISTS tenant_member_roles (
    tenant_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role_id TEXT NOT NULL,
    PRIMARY KEY (tenant_id, user_id, role_id)
);
CREATE TABLE IF NOT EXISTS role_permissions (
    tenant_id TEXT NOT NULL,
    role_id TEXT NOT NULL,
    permission TEXT NOT NULL,
    PRIMARY KEY (tenant_id, role_id, permission)
);

CREATE TABLE IF NOT EXISTS organization_departments (
    tenant_id TEXT NOT NULL,
    id TEXT NOT NULL,
    parent_id TEXT,
    name TEXT NOT NULL,
    leader TEXT NOT NULL DEFAULT '',
    phone TEXT NOT NULL DEFAULT '',
    email TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, id)
);

CREATE INDEX IF NOT EXISTS organization_departments_parent_idx
    ON organization_departments (tenant_id, parent_id, sort_order, id);

CREATE TABLE IF NOT EXISTS organization_posts (
    tenant_id TEXT NOT NULL,
    id TEXT NOT NULL,
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'active',
    remark TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, id),
    UNIQUE (tenant_id, code)
);

CREATE INDEX IF NOT EXISTS organization_posts_status_idx
    ON organization_posts (tenant_id, status, sort_order, id);
