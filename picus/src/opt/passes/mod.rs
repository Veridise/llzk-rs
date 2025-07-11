mod consolidate_var_names;
mod ensure_max_size;
mod fold;

pub use consolidate_var_names::ConsolidateVarNamesPass;
pub use ensure_max_size::EnsureMaxExprSizePass;
pub use fold::FoldExprsPass;
