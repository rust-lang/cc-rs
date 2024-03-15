use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct PreLinkArgs(
    /// First field in the linker name,
    /// second field is the args.
    #[serde(with = "tuple_vec_map")]
    pub Vec<(String, Vec<String>)>,
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
    pub pre_link_args: Option<PreLinkArgs>,
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct RustcTargetSpecs(
    /// First field in the tuple is the rustc target
    #[serde(with = "tuple_vec_map")]
    pub Vec<(String, TargetSpec)>,
);
