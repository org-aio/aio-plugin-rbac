mod members;
mod organization;
mod policy;
mod roles;
mod routes;
mod service;

#[cfg(test)]
mod tests;

pub use service::AccessControlService;

use anyhow::{Context as _, Result};
use axum::Router;
use dill::CatalogBuilder;

pub fn register(builder: &mut CatalogBuilder) -> Result<()> {
    builder.add_value(AccessControlService::from_env()?);
    Ok(())
}

pub fn service(catalog: &dill::Catalog) -> Result<std::sync::Arc<AccessControlService>> {
    catalog
        .get_one::<AccessControlService>()
        .context("权限服务未注册")
}

pub fn router(catalog: &dill::Catalog) -> Result<Router> {
    let access = service(catalog)?;
    let identity = aio_plugin_identity_server::service(catalog)?;
    Ok(routes::router(access, identity))
}
