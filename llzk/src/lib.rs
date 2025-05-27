use llzk_sys::llzkRegisterAllDialects;
use melior::dialect::DialectRegistry;

pub mod builder;
pub mod dialect;
#[cfg(test)]
mod test;

pub fn register_all_llzk_dialects(registry: &DialectRegistry) {
    unsafe { llzkRegisterAllDialects(registry.to_raw()) }
}
