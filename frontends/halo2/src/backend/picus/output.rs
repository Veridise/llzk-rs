use std::{cell::Ref, collections::HashMap, fmt, marker::PhantomData, rc::Rc};

use rug::Integer;

use super::{expr::PicusExpr, lowering::PicusModuleRef, vars::VarStr};
use crate::backend::func::{ArgNo, FieldId, FuncIO};
use crate::halo2::{Field, PrimeField};

pub struct PicusFelt(Integer);

impl<F: Field> From<F> for PicusFelt {
    fn from(value: F) -> Self {
        let s = format!("{:?}", value);
        Self(Integer::from_str_radix(&s[2..], 16).expect("parse felt hex representation"))
    }
}

impl PicusFelt {
    pub fn prime<F: Field>() -> Self {
        let mut f = Self::from(-F::ONE);
        f.0 += 1;
        f
    }
}

impl fmt::Display for PicusFelt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct PicusOutput<F> {
    modules: Vec<PicusModule>,
    _marker: PhantomData<F>,
}

impl<'a, F> From<Vec<PicusModuleRef>> for PicusOutput<F> {
    fn from(modules: Vec<PicusModuleRef>) -> Self {
        Self {
            modules: modules.into_iter().map(Into::into).collect(),
            _marker: Default::default(),
        }
    }
}

impl<F: PrimeField> fmt::Display for PicusOutput<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "(prime-number {})", PicusFelt::prime::<F>())?;
        for module in &self.modules {
            writeln!(f, "{module}")?;
        }
        write!(f, "")
    }
}

#[derive(Clone)]
struct PicusConstraint(PicusExpr);

impl fmt::Display for PicusConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(assert {})", self.0)
    }
}

trait VarIO {
    fn is_input(&self) -> bool;
    fn is_output(&self) -> bool;
}

#[derive(Clone, Copy, Hash, Eq, PartialEq)]
pub enum VarKey {
    IO(FuncIO),
    Temp(usize),
}

impl VarIO for VarKey {
    fn is_input(&self) -> bool {
        match self {
            VarKey::IO(func_io) => match func_io {
                FuncIO::Arg(_) => true,
                _ => false,
            },
            _ => false,
        }
    }

    fn is_output(&self) -> bool {
        match self {
            VarKey::IO(func_io) => match func_io {
                FuncIO::Field(_) => true,
                _ => false,
            },
            _ => false,
        }
    }
}

impl<K: VarIO, V> VarIO for (&K, &V) {
    fn is_input(&self) -> bool {
        self.0.is_input()
    }

    fn is_output(&self) -> bool {
        self.0.is_output()
    }
}

impl<T: Into<FuncIO>> From<T> for VarKey {
    fn from(value: T) -> Self {
        Self::IO(value.into())
    }
}

impl From<usize> for VarKey {
    fn from(value: usize) -> Self {
        Self::Temp(value)
    }
}

#[derive(Clone)]
pub struct PicusModule {
    name: String,
    constraints: Vec<PicusConstraint>,
    calls: Vec<PicusExpr>,
    vars: HashMap<VarKey, VarStr>,
}

impl From<PicusModuleRef> for PicusModule {
    fn from(value: PicusModuleRef) -> Self {
        value.borrow().clone()
    }
}

impl From<String> for PicusModule {
    fn from(name: String) -> Self {
        Self {
            name,
            constraints: Default::default(),
            calls: Default::default(),
            vars: Default::default(),
        }
    }
}

impl PicusModule {
    pub fn shared(name: String, n_inputs: usize, n_outputs: usize) -> PicusModuleRef {
        Rc::new(Self::new(name, n_inputs, n_outputs).into())
    }

    pub fn new(name: String, n_inputs: usize, n_outputs: usize) -> Self {
        let mut m = Self::from(name);
        (0..n_inputs).map(ArgNo::from).for_each(|a| {
            m.vars.insert(a.into(), a.into());
        });

        (0..n_outputs).map(FieldId::from).for_each(|f| {
            m.vars.insert(f.into(), f.into());
        });
        m
    }

    pub fn add_constraint(&mut self, constraint: PicusExpr) {
        self.constraints.push(PicusConstraint(constraint))
    }

    pub fn constraints_len(&self) -> usize {
        self.constraints.len()
    }

    pub fn add_call(&mut self, expr: PicusExpr) {
        self.calls.push(expr)
    }

    pub fn add_var(&mut self, key: Option<FuncIO>) -> VarStr {
        let tmp_no = self.vars.len();
        let key = key.map(Into::into).unwrap_or_else(|| tmp_no.into());
        if self.vars.contains_key(&key) {
            return self.vars[&key].clone();
        }
        self.vars.insert(key, key.into());
        key.into()
    }
}

impl fmt::Display for PicusModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "(begin-module {})", self.name)?;
        for i in self.vars.iter().filter(VarIO::is_input) {
            writeln!(f, "(input {})", i.1)?;
        }
        for o in self.vars.iter().filter(VarIO::is_output) {
            writeln!(f, "(output {})", o.1)?;
        }
        for c in &self.constraints {
            writeln!(f, "{c}")?;
        }
        for c in &self.calls {
            writeln!(f, "{c}")?;
        }
        writeln!(f, "(end-module)")
    }
}
