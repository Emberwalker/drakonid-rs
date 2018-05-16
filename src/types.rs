use std::sync::Arc;

use config;
use typemap;

// Newtype around Config to support ShareMap
pub struct ConfigMarker;

impl typemap::Key for ConfigMarker {
    type Value = Arc<config::Config>;
}
