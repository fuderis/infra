#![allow(unused_imports)]
use crate::prelude::DynError;
use macron::{Display, Error, From};

// The application error
#[derive(Debug, Display, Error, From)]
pub enum Error {
    #[from(skip)]
    #[display(fmt = "Host '{0}' not configured in settings.toml")]
    UnknownHost(String),

    #[display(fmt = "Specify the host (-h) for this action")]
    SpecifyHost,

    #[from(skip)]
    #[display(fmt = "Operational error: {0}")]
    Operational(String),
}
