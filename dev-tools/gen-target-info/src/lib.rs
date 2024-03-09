mod target_specs;
pub use target_specs::*;

mod read;
pub use read::get_target_specs_from_json;

mod write;
pub use write::*;
