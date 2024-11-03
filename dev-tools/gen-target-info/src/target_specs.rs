use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct PreLinkArgs(
    /// First field in the linker name,
    /// second field is the args.
    pub BTreeMap<String, Vec<String>>,
);

#[derive(Debug, Deserialize)]
#[serde(rename_all(deserialize = "kebab-case"))]
pub struct TargetSpec {
    pub arch: String,
    pub llvm_target: String,
    /// link env to remove, mostly for apple
    pub link_env_remove: Option<Vec<String>>,
    /// link env to set, mostly for apple, e.g. `ZERO_AR_DATE=1`
    pub link_env: Option<Vec<String>>,
    pub os: Option<String>,
    /// `apple`, `pc`
    pub vendor: Option<String>,
    pub env: Option<String>,
    pub abi: Option<String>,
    pub target_pointer_width: String,
    pub pre_link_args: Option<PreLinkArgs>,
    #[serde(skip)]
    pub cfgs: Cfgs,
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct RustcTargetSpecs(
    /// First field in the tuple is the rustc target
    pub BTreeMap<String, TargetSpec>,
);

/// Potentially useful values from:
/// https://doc.rust-lang.org/reference/conditional-compilation.html
///
/// That are not directly / easily exposed in `TargetSpec`.
#[derive(Debug, Default)]
pub struct Cfgs {
    pub target_features: Vec<String>,
    pub target_families: Vec<String>,
    pub target_endian: String,
    pub target_atomics: Vec<String>,
    pub target_thread_local: bool,
}

impl Cfgs {
    pub fn parse(cfgs: &[String]) -> Self {
        let mut target_features = vec![];
        let mut target_families = vec![];
        let mut target_endian = None;
        let mut target_atomics = vec![];
        let mut target_thread_local = false;
        for cfg in cfgs {
            let (name, value) = cfg
                .split_once('=')
                .map(|(n, v)| (n.trim(), Some(v.trim().trim_matches('"'))))
                .unwrap_or((cfg.trim(), None));

            match (name, value) {
                ("target_feature", Some(value)) => target_features.push(value.to_string()),
                ("target_family", Some(value)) => target_families.push(value.to_string()),
                ("target_endian", Some(value)) => target_endian = Some(value.to_string()),
                ("target_has_atomic", Some(value)) => target_atomics.push(value.to_string()),
                ("target_thread_local", None) => target_thread_local = true,
                _ => {} // Ignore the rest
            }
        }

        Self {
            target_features,
            target_families,
            target_endian: target_endian.expect("must have target_endian cfg"),
            target_atomics,
            target_thread_local,
        }
    }
}
