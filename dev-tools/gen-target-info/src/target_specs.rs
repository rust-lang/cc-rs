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
    pub pre_link_args: Option<PreLinkArgs>,
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct RustcTargetSpecs(
    /// First field in the tuple is the rustc target
    pub BTreeMap<String, TargetSpec>,
);
