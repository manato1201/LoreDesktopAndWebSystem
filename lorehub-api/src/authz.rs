//! Path-based access-control resolution.
//!
//! `AppState.access_entries` maps a directory/file path to the list of
//! principals (users/teams) granted specific `PermissionLevel`s on it. Until
//! this module existed, nothing outside the three
//! `/api/access-control/entries*` CRUD handlers ever consulted that data —
//! any authenticated user could read/write/lock any path regardless of what
//! was configured. `check_path_permission` is the single function every
//! path-guarded handler calls to make that data actually mean something.
use crate::models::{MemberRole, OrgMember, PermissionLevel, PrincipalType};
use crate::state::AppState;

/// `path` and every ancestor directory of `path`, most specific first,
/// ending at the top-level path segment (there is no synthetic empty-string
/// "root" entry — `access_entries` never keys on `""`). E.g.
/// `"Assets/Characters/hero_rig.fbx"` yields
/// `["Assets/Characters/hero_rig.fbx", "Assets/Characters", "Assets"]`.
fn path_ancestors(path: &str) -> Vec<String> {
    let mut ancestors = Vec::new();
    let mut current = path.to_string();
    loop {
        ancestors.push(current.clone());
        match current.rfind('/') {
            Some(idx) => current.truncate(idx),
            None => break,
        }
    }
    ancestors
}

/// Resolves whether `user` may act on `path` at `required` permission level.
///
/// Algorithm:
/// 1. `Owner`/`Admin` role always passes — an org-level override so admins
///    don't also need to be a member of every team to do their job.
/// 2. Otherwise walk `path`'s ancestor chain from most to least specific
///    (see [`path_ancestors`]). The **first** ancestor that has any
///    `AccessEntry` at all in `access_entries` wins; a less-specific
///    ancestor's entries are never consulted once a more specific one is
///    found, even if the more specific one would otherwise deny.
/// 3. If **no** ancestor at any level has entries, default to **allow** —
///    an unconfigured path has no restriction configured on it, matching
///    the app's pre-existing de facto fully-open behavior so unconfigured
///    demo repos/paths keep working exactly as before.
/// 4. If the winning ancestor has entries, `user` must match at least one
///    (`principalType == User && principal == user.name`, or
///    `principalType == Team && user.teams` contains `principal`) whose
///    `permissions` include `required`. No match => deny.
///
/// ## `repo_slug` and the scope of `access_entries`
///
/// `repo_slug` is accepted for interface completeness (callers already have
/// it, and a future repo-scoped ACL model would need it), but is
/// deliberately **not** used to key into `access_entries` here. Unlike
/// `tree`/`commits`/`branches` (which *were* refactored this session from a
/// single shared dataset into a `HashMap<repo_slug, _>` because each repo is
/// supposed to have independent VCS history), `access_entries` was checked
/// and found to still be `HashMap<path, Vec<AccessEntry>>` — a single flat,
/// org-wide map — but this reflects the actual, intentional product design,
/// not a leftover instance of that bug class:
///
/// - The architecture doc's `AppState` class diagram documents it as
///   `access_entries: Record<path, AccessEntry[]>` with no repo dimension.
/// - LoreForge Server Admin's node-editor "Apply" flow
///   (`PermissionConfigController::applyToServer`) has **no concept of a
///   repository at all** — it PUTs a flat `{ path: AccessEntry[] }` graph to
///   `/api/access-control/entries` (no `{slug}` in the route), by design.
/// - LoreHub Web's Access Control page has a per-repo tab selector in its
///   UI, but calls `getAccessEntries()` with no repo argument regardless of
///   the selected tab — every tab reads the same org-wide table.
///
/// Restructuring this into a genuinely repo-scoped model would require
/// adding a `{slug}` segment (or similar) to
/// `GET`/`PUT /api/access-control/entries`, which neither client sends
/// today — out of scope for this backend-only authorization task per the
/// brief ("no frontend/desktop-client changes needed"). Left as a follow-up.
pub fn check_path_permission(
    state: &AppState,
    user: &OrgMember,
    _repo_slug: &str,
    path: &str,
    required: PermissionLevel,
) -> bool {
    if matches!(user.role, MemberRole::Owner | MemberRole::Admin) {
        return true;
    }

    for ancestor in path_ancestors(path) {
        let Some(entries) = state.access_entries.get(&ancestor) else {
            continue;
        };
        if entries.is_empty() {
            // An empty entry list doesn't count as "configured" — keep
            // walking up to the next ancestor.
            continue;
        }
        return entries.iter().any(|entry| {
            let principal_matches = match entry.principal_type {
                PrincipalType::User => entry.principal == user.name,
                PrincipalType::Team => user.teams.iter().any(|t| t == &entry.principal),
            };
            principal_matches && entry.permissions.contains(&required)
        });
    }

    // No ancestor at any level has any configured entries.
    true
}
