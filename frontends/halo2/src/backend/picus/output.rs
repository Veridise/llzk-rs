use std::collections::HashSet;
use std::ops::{Add, AddAssign};
use std::{collections::HashMap, fmt, marker::PhantomData, rc::Rc};

use anyhow::Result;
use rug::Integer;

use super::stmt::{self, CallLike, PicusStmt};
use super::vars::VarKey;
use super::{expr::PicusExpr, lowering::PicusModuleRef, vars::VarStr};
use crate::backend::func::{ArgNo, FieldId, FuncIO};
use crate::backend::picus::vars::VarIO;
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

impl<F> PicusOutput<F> {
    fn module_names(&self) -> HashSet<String> {
        self.modules.iter().map(|m| m.name.clone()).collect()
    }

    pub fn merge(&mut self, other: PicusOutput<F>) -> Result<()> {
        let collisions: HashSet<String> = self
            .module_names()
            .intersection(&other.module_names())
            .map(Clone::clone)
            .collect();
        // Maps the old name to the new one
        let mut renames: HashMap<String, String> = Default::default();

        let renamed = other
            .modules
            .into_iter()
            .map(|m| -> Result<PicusModule> {
                if !collisions.contains(&m.name) {
                    return Ok(m);
                }

                let new_name = (0..)
                    .find_map(|i| {
                        let new_name = format!("{}{i}", m.name);
                        if collisions.contains(&new_name) {
                            return None;
                        }
                        Some(new_name)
                    })
                    .ok_or_else(|| anyhow::anyhow!("Failed to find a new name"))?;
                let mut m = m;
                renames.insert(m.name, new_name.clone());
                m.name = new_name;
                Ok(m)
            })
            // Collect the modules to make a barrier since the next step needs the full list of
            // renames
            .collect::<Result<Vec<_>>>()?;
        let renames = renames;
        self.modules.extend(renamed.into_iter().map(|m| {
            let mut m = m;

            m.stmts = m
                .stmts
                .into_iter()
                .map(|s| {
                    if let Some(call) = s.as_call() {
                        if renames.contains_key(call.callee()) {
                            return call.with_new_callee(renames[call.callee()].clone());
                        }
                    }
                    s
                })
                .collect();

            m
        }));

        Ok(())
    }
}

impl<F> Add for PicusOutput<F> {
    type Output = Result<PicusOutput<F>>;

    fn add(self, rhs: Self) -> Self::Output {
        let mut lhs = self;
        lhs.merge(rhs)?;
        Ok(lhs)
    }
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
pub struct PicusModule {
    name: String,
    stmts: Vec<PicusStmt>,
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
            stmts: Default::default(),
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
        self.stmts.push(stmt::constrain(constraint))
    }

    pub fn constraints_len(&self) -> usize {
        self.stmts.iter().filter(|s| s.is_constraint()).count()
    }

    pub fn add_call(&mut self, stmt: PicusStmt) {
        self.stmts.push(stmt)
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
        for c in &self.stmts {
            writeln!(f, "{c}")?;
        }
        writeln!(f, "(end-module)")
    }
}
