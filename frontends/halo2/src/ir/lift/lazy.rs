use std::{
    any::{type_name, TypeId},
    collections::HashMap,
    sync::{Mutex, MutexGuard},
};

use ff::{Field, PrimeField};

use crate::arena::{BumpArena, Index};

use super::inner::LiftInner;

lazy_static::lazy_static! {
     static ref ZERO_INDEX: Mutex<LazyConstants> = Mutex::new(Default::default());
     static ref ONE_INDEX: Mutex<LazyConstants> = Mutex::new(Default::default());
     static ref TWO_INV_INDEX: Mutex<LazyConstants> = Mutex::new(Default::default());
     static ref MULT_GEN_INDEX: Mutex<LazyConstants> = Mutex::new(Default::default());
     static ref ROOT_INDEX: Mutex<LazyConstants> = Mutex::new(Default::default());
     static ref ROOT_INV_INDEX: Mutex<LazyConstants> = Mutex::new(Default::default());
     static ref DELTA_INDEX: Mutex<LazyConstants> = Mutex::new(Default::default());
}

/// Keeps track of a constant per type of F we used.
#[derive(Default)]
struct LazyConstants {
    constants: HashMap<TypeId, Index>,
}

impl LazyConstants {
    pub fn find<F: 'static>(&self) -> Option<&Index> {
        log::debug!("Looking for a constant of type {}", type_name::<F>());
        let id = TypeId::of::<F>();
        self.constants.get(&id)
    }

    pub fn insert<F: 'static>(&mut self, idx: Index) {
        log::debug!("Storing a constant of type {}", type_name::<F>());
        self.constants.insert(TypeId::of::<F>(), idx);
    }
}

fn lazy_init_const<'a, 'b: 'a, F: 'static, FO: 'a>(
    mut guard: MutexGuard<LazyConstants>,
    value: F,
    arena: &'b mut MutexGuard<BumpArena>,
    mut cb: impl FnMut(&'b mut MutexGuard<BumpArena>, &Index) -> FO,
) -> FO {
    match guard.find::<F>() {
        Some(idx) => cb(arena, idx),
        None => {
            let inner = LiftInner::r#const(value);
            let idx = arena.insert(inner);
            guard.insert::<F>(idx);
            cb(arena, &idx)
        }
    }
}

pub fn lazy_init_zero<'a, 'b: 'a, F: Field + 'static, FO: 'a>(
    arena: &'b mut MutexGuard<BumpArena>,
    cb: impl FnMut(&'b mut MutexGuard<BumpArena>, &Index) -> FO,
) -> FO {
    lazy_init_const(ZERO_INDEX.lock().unwrap(), F::ZERO, arena, cb)
}

pub fn lazy_init_one<'a, 'b: 'a, F: Field + 'static, FO: 'a>(
    arena: &'b mut MutexGuard<BumpArena>,
    cb: impl FnMut(&'b mut MutexGuard<BumpArena>, &Index) -> FO,
) -> FO {
    lazy_init_const(ONE_INDEX.lock().unwrap(), F::ONE, arena, cb)
}

pub fn lazy_init_two_inv<'a, 'b: 'a, F: PrimeField + 'static, FO: 'a>(
    arena: &'b mut MutexGuard<BumpArena>,
    cb: impl FnMut(&'b mut MutexGuard<BumpArena>, &Index) -> FO,
) -> FO {
    lazy_init_const(TWO_INV_INDEX.lock().unwrap(), F::TWO_INV, arena, cb)
}

pub fn lazy_init_mult_gen<'a, 'b: 'a, F: PrimeField + 'static, FO: 'a>(
    arena: &'b mut MutexGuard<BumpArena>,
    cb: impl FnMut(&'b mut MutexGuard<BumpArena>, &Index) -> FO,
) -> FO {
    lazy_init_const(
        MULT_GEN_INDEX.lock().unwrap(),
        F::MULTIPLICATIVE_GENERATOR,
        arena,
        cb,
    )
}

pub fn lazy_init_root<'a, 'b: 'a, F: PrimeField + 'static, FO: 'a>(
    arena: &'b mut MutexGuard<BumpArena>,
    cb: impl FnMut(&'b mut MutexGuard<BumpArena>, &Index) -> FO,
) -> FO {
    lazy_init_const(ROOT_INDEX.lock().unwrap(), F::ROOT_OF_UNITY, arena, cb)
}

pub fn lazy_init_root_inv<'a, 'b: 'a, F: PrimeField + 'static, FO: 'a>(
    arena: &'b mut MutexGuard<BumpArena>,
    cb: impl FnMut(&'b mut MutexGuard<BumpArena>, &Index) -> FO,
) -> FO {
    lazy_init_const(
        ROOT_INV_INDEX.lock().unwrap(),
        F::ROOT_OF_UNITY_INV,
        arena,
        cb,
    )
}

pub fn lazy_init_delta<'a, 'b: 'a, F: PrimeField + 'static, FO: 'a>(
    arena: &'b mut MutexGuard<BumpArena>,
    cb: impl FnMut(&'b mut MutexGuard<BumpArena>, &Index) -> FO,
) -> FO {
    lazy_init_const(DELTA_INDEX.lock().unwrap(), F::DELTA, arena, cb)
}
