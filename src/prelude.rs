pub use crate::{error::Error, settings::Settings};

pub use std::result::Result as StdResult;

/// The dynamic error type
pub type DynError = Box<dyn std::error::Error + Send + Sync + 'static>;
/// The short result alias
pub type Result<T> = StdResult<T, DynError>;

pub use atoman::*;
pub use colored::*;
pub use macron::*;

pub use serde::{Deserialize, Serialize};
pub use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
