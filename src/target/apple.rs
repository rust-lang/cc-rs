use super::TargetInfo;

impl TargetInfo<'_> {
    pub(crate) fn apple_sdk_name(&self) -> &'static str {
        match (self.os, self.abi) {
            ("macos", "") => "macosx",
            ("ios", "") => "iphoneos",
            ("ios", "sim") => "iphonesimulator",
            ("ios", "macabi") => "macosx",
            ("tvos", "") => "appletvos",
            ("tvos", "sim") => "appletvsimulator",
            ("watchos", "") => "watchos",
            ("watchos", "sim") => "watchsimulator",
            ("visionos", "") => "xros",
            ("visionos", "sim") => "xrsimulator",
            (os, _) => panic!("invalid Apple target OS {}", os),
        }
    }
}
