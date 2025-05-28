use llzk_sys::llzkRegisterAllDialects;
use melior::dialect::DialectRegistry;

pub mod builder;
pub mod dialect;
pub mod error;
pub mod symbol_ref;
#[cfg(test)]
mod test;
pub mod utils;
pub mod value_range;

pub fn register_all_llzk_dialects(registry: &DialectRegistry) {
    unsafe { llzkRegisterAllDialects(registry.to_raw()) }
}
