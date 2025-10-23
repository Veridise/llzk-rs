#![warn(rustdoc::broken_intra_doc_links)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]

#[macro_use]
pub mod display;
pub mod expr;
pub mod felt;
pub mod ident;
mod module;
pub mod opt;
mod program;
pub mod stmt;
pub mod vars;

pub use module::{Module, ModuleLike, ModuleRef, ModuleWithVars};
pub use program::Program;
