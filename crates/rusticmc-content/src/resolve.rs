//! Dependency auto-resolution: a version's required dependencies, theirs, and so on, ordered so
//! each comes after what it needs; optional ones are left to the player; incompatible ones
//! already installed stop the install.
use crate::error::{ContentError, Result};
use crate::ledger::Ledger;
use crate::model::{ContentVersion, DependencyKind};

/// What a dependency is looked for against.
#[derive(Clone, Debug, Default)]
pub struct ResolveContext {
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
}

/// Where dependencies are looked up (a source).
#[async_trait::async_trait]
pub trait DependencySource: Send + Sync {
    async fn by_version_id(&self, id: &str) -> Result<ContentVersion>;
    /// The version of `project` to install for `ctx`, newest first; None when none fits.
    async fn best_for_project(
        &self,
        project: &str,
        ctx: &ResolveContext,
    ) -> Result<Option<ContentVersion>>;
}

/// The versions to install for `root`: its required dependencies first (depth first, each
/// once), then `root`. Projects already in `installed` are left out. Most 64 projects.
pub async fn resolve(
    root: ContentVersion,
    source: &dyn DependencySource,
    installed: &Ledger,
    ctx: &ResolveContext,
) -> Result<Vec<ContentVersion>> {
    let mut order: Vec<ContentVersion> = Vec::new();
    let mut visiting: Vec<String> = Vec::new();
    visit(root, source, installed, ctx, &mut order, &mut visiting).await?;
    Ok(order)
}

fn visit<'a>(
    v: ContentVersion,
    source: &'a dyn DependencySource,
    installed: &'a Ledger,
    ctx: &'a ResolveContext,
    order: &'a mut Vec<ContentVersion>,
    visiting: &'a mut Vec<String>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
    Box::pin(async move {
        if order.iter().any(|o| o.project_id == v.project_id) {
            return Ok(());
        }
        if visiting.contains(&v.project_id) {
            // A cycle of required dependencies: install them together.
            return Ok(());
        }
        if order.len() + visiting.len() > 64 {
            return Err(ContentError::Dependency(
                "more than 64 dependencies: refusing to go on".into(),
            ));
        }
        for d in &v.dependencies {
            if d.kind == DependencyKind::Incompatible
                && let Some(p) = &d.project_id
                && installed.find_project(v.source, p).is_some()
            {
                return Err(ContentError::Dependency(format!(
                    "{} is incompatible with the installed {p}",
                    v.name
                )));
            }
        }
        visiting.push(v.project_id.clone());
        for d in v
            .dependencies
            .iter()
            .filter(|d| d.kind == DependencyKind::Required)
        {
            if let Some(p) = &d.project_id
                && (installed.find_project(v.source, p).is_some()
                    || order.iter().any(|o| &o.project_id == p))
            {
                continue;
            }
            let dep = match (&d.version_id, &d.project_id) {
                (Some(id), _) => source.by_version_id(id).await?,
                (None, Some(p)) => source.best_for_project(p, ctx).await?.ok_or_else(|| {
                    ContentError::Dependency(format!(
                        "{} needs {p}, which has no version for {}",
                        v.name,
                        ctx.game_versions.join(", ")
                    ))
                })?,
                (None, None) => continue,
            };
            if installed
                .find_project(dep.source, &dep.project_id)
                .is_some()
            {
                continue;
            }
            visit(dep, source, installed, ctx, order, visiting).await?;
        }
        visiting.retain(|p| p != &v.project_id);
        order.push(v);
        Ok(())
    })
}
