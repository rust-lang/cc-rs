//! The environment a [`Build`](crate::Build) reads its configuration from and
//! runs its tools in.

use std::{env, ffi::OsStr, fmt, process::Command, sync::Arc};

use crate::utilities::OnceLock;

/// Environment variables, in the order they are applied.
pub(crate) type EnvVars = [(Arc<OsStr>, Arc<OsStr>)];

/// The environment of a [`Build`](crate::Build): a snapshot of the process
/// environment, taken on the first read and used for cc's own lookups, then
/// the entries set with [`Build::env`](crate::Build::env), which win over it in
/// child processes.
#[derive(Clone, Debug, Default)]
pub(crate) struct BuildEnv {
    /// Private, so that nothing can read the environment without taking the
    /// snapshot first.
    inherited: OnceLock<EnvSnapshot>,
    pub(crate) explicit: Vec<(Arc<OsStr>, Arc<OsStr>)>,
}

impl BuildEnv {
    /// An environment whose snapshot is already taken.
    pub(crate) fn with_inherited(
        inherited: EnvSnapshot,
        explicit: Vec<(Arc<OsStr>, Arc<OsStr>)>,
    ) -> Self {
        let cell = OnceLock::new();
        cell.get_or_init(|| inherited);
        Self {
            inherited: cell,
            explicit,
        }
    }

    /// The process environment as it was when this was first called.
    pub(crate) fn inherited(&self) -> &EnvSnapshot {
        self.inherited.get_or_init(EnvSnapshot::capture)
    }

    /// Make `cmd` run in this environment. Like [`EnvSnapshot::apply`], this
    /// clears anything set on `cmd`'s environment before.
    pub(crate) fn apply(&self, cmd: &mut Command) {
        self.inherited().apply(cmd);
        cmd.envs(pairs(&self.explicit));
    }
}

/// A copy of the process environment. Clones share the copy.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct EnvSnapshot {
    vars: Arc<EnvVars>,
}

impl EnvSnapshot {
    /// Copy the process environment as it is now.
    pub(crate) fn capture() -> Self {
        Self {
            vars: env::vars_os()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        }
    }

    #[cfg(test)]
    pub(crate) fn from_pairs(vars: &[(&str, &str)]) -> Self {
        Self {
            vars: vars
                .iter()
                .map(|(key, value)| (OsStr::new(key).into(), OsStr::new(value).into()))
                .collect(),
        }
    }

    /// Look up `key` the way [`env::var_os`] would have when the snapshot was
    /// taken.
    pub(crate) fn get(&self, key: &OsStr) -> Option<&Arc<OsStr>> {
        self.vars
            .iter()
            .find(|(k, _)| is_same_key(k, key))
            .map(|(_, value)| value)
    }

    /// Make `cmd` run in this environment rather than the current one.
    ///
    /// This clears the environment `cmd` would inherit and sets every variable
    /// of the snapshot, so the child does not depend on the process environment
    /// at the time it is spawned, which another thread may be changing. It also
    /// clears anything set on `cmd`'s environment before, so call it first.
    pub(crate) fn apply(&self, cmd: &mut Command) {
        cmd.env_clear();
        cmd.envs(pairs(&self.vars));
    }
}

impl fmt::Debug for EnvSnapshot {
    // The values may hold secrets, and a `Build` can end up in a log.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EnvSnapshot(<{} variables>)", self.vars.len())
    }
}

fn pairs(vars: &EnvVars) -> impl Iterator<Item = (&OsStr, &OsStr)> {
    vars.iter().map(|(key, value)| (&**key, &**value))
}

/// Environment variable names are case-insensitive on Windows.
fn is_same_key(a: &OsStr, b: &OsStr) -> bool {
    if cfg!(windows) {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn snapshot(vars: &[(&str, &str)]) -> EnvSnapshot {
        EnvSnapshot::from_pairs(vars)
    }

    fn envs(cmd: &Command) -> Vec<(OsString, Option<OsString>)> {
        cmd.get_envs()
            .map(|(key, value)| (key.to_owned(), value.map(OsStr::to_owned)))
            .collect()
    }

    #[test]
    fn apply_replaces_the_whole_environment() {
        let mut cmd = Command::new("cc");
        cmd.env("CC_TEST_SET_BEFORE", "1")
            .env_remove("CC_TEST_REMOVED_BEFORE");
        snapshot(&[("CC_TEST_IN_SNAPSHOT", "1")]).apply(&mut cmd);
        assert_eq!(
            envs(&cmd),
            [("CC_TEST_IN_SNAPSHOT".into(), Some("1".into()))]
        );
    }

    #[test]
    fn build_env_wins_over_snapshot() {
        let env = BuildEnv::with_inherited(
            snapshot(&[("CC_TEST_ORDER", "inherited")]),
            vec![(
                OsStr::new("CC_TEST_ORDER").into(),
                OsStr::new("explicit").into(),
            )],
        );
        let mut cmd = Command::new("cc");
        env.apply(&mut cmd);
        assert_eq!(
            envs(&cmd),
            [("CC_TEST_ORDER".into(), Some("explicit".into()))]
        );
    }

    #[test]
    fn snapshot_lookup_shares_the_value() {
        let snapshot = snapshot(&[("CC_TEST_SHARED", "a")]);
        let value = snapshot.get(OsStr::new("CC_TEST_SHARED")).unwrap();
        assert!(Arc::ptr_eq(value, &snapshot.vars[0].1));
    }

    #[test]
    fn snapshot_lookup_follows_the_platform_case_rule() {
        let snapshot = snapshot(&[("Path", "a")]);
        assert_eq!(
            snapshot.get(OsStr::new("Path")).map(|value| &**value),
            Some(OsStr::new("a"))
        );
        let other_case = snapshot.get(OsStr::new("PATH")).map(|value| &**value);
        if cfg!(windows) {
            assert_eq!(other_case, Some(OsStr::new("a")));
        } else {
            assert_eq!(other_case, None);
        }
    }

    #[test]
    fn snapshot_debug_hides_values() {
        let debug = format!("{:?}", snapshot(&[("CC_TEST_SECRET", "hunter2")]));
        assert!(!debug.contains("hunter2"), "{debug}");
    }
}
