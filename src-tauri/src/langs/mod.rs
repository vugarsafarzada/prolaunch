//! Per-language project support: scaffolds, toolchain bins and project readers.
//! Shared infrastructure stays in [`crate::common`].

pub(crate) mod dart;
pub(crate) mod go;
pub(crate) mod java;
pub(crate) mod node;
pub(crate) mod php;
pub(crate) mod python;
pub(crate) mod ruby;
