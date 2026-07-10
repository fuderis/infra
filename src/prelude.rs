#![allow(unused_imports)]
pub use crate::{APP_NAME, APP_VERSION, error::Error, settings::Settings};

// Result alias:
pub use std::result::Result as StdResult;
pub type DynError = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type Result<T> = StdResult<T, DynError>;

// Utilities:
pub use atoman::*;
pub use macron::*;

// Application:
pub use colored::*;

// STD & Tokio:
pub use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

// Serialization:
pub use serde::{Deserialize, Serialize};
