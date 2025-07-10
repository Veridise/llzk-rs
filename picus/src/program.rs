use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
    ops::Add,
};

use crate::{
    display::{Display, TextRepresentable, TextRepresentation},
    felt::{Felt, IntoPrime},
    stmt::traits::{CallLike as _, MaybeCallLike as _},
    vars::VarKind,
    Module, ModuleRef,
};
use anyhow::{anyhow, Result};

pub struct Program<F, K: VarKind> {
    prime: PrimeNumber,
    modules: Vec<Module<K>>,
    _marker: PhantomData<F>,
}

impl<F, K: VarKind> Program<F, K> {
    pub fn modules(&self) -> &[Module<K>] {
        &self.modules
    }

    pub fn modules_mut(&mut self) -> &mut [Module<K>] {
        &mut self.modules
    }

    fn module_names(&self) -> HashSet<&str> {
        self.modules.iter().map(|m| m.name.as_str()).collect()
    }

    pub fn display(&self) -> Display<F, K> {
        Display::new(self)
    }

    pub fn merge(&mut self, other: Program<F, K>) -> Result<()> {
        let collisions: HashSet<String> = self
            .module_names()
            .intersection(&other.module_names())
            .map(|s| (*s).to_owned())
            .collect();
        // Maps the old name to the new one
        let mut renames: HashMap<String, String> = Default::default();

        let renamed = other
            .modules
            .into_iter()
            .map(|m| -> Result<Module<K>> {
                if !collisions.contains(m.name.as_str()) {
                    return Ok(m);
                }

                let new_name = (0..)
                    .find_map(|i| {
                        let new_name = format!("{}{i}", *m.name);
                        if collisions.contains(new_name.as_str()) {
                            return None;
                        }
                        Some(new_name)
                    })
                    .ok_or_else(|| anyhow!("Failed to find a new name"))?;
                let mut m = m;
                renames.insert(m.name.to_string(), new_name.clone());
                *m.name = new_name;
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

impl<F, K: VarKind> Add for Program<F, K> {
    type Output = Result<Program<F, K>>;

    fn add(self, rhs: Self) -> Self::Output {
        let mut lhs = self;
        lhs.merge(rhs)?;
        Ok(lhs)
    }
}

impl<F: IntoPrime, K: VarKind + Clone> From<Vec<ModuleRef<K>>> for Program<F, K> {
    fn from(modules: Vec<ModuleRef<K>>) -> Self {
        Self {
            prime: PrimeNumber(Felt::prime::<F>()),
            modules: modules.into_iter().map(Into::into).collect(),
            _marker: Default::default(),
        }
    }
}

struct PrimeNumber(Felt);

impl TextRepresentable for PrimeNumber {
    fn to_repr(&self) -> TextRepresentation {
        owned_list!("prime-number", &self.0).break_line()
    }

    fn width_hint(&self) -> usize {
        15 + self.0.width_hint()
    }
}

type TR<'a> = TextRepresentation<'a>;

impl<F: IntoPrime, K: VarKind> TextRepresentable for Program<F, K> {
    fn to_repr(&self) -> TextRepresentation {
        self.modules
            .iter()
            .fold(owned_list!(&self.prime), |acc, m| {
                acc + TR::br() + m.to_repr()
            })
            .with_punct("".into())
    }

    fn width_hint(&self) -> usize {
        self.prime.width_hint()
    }
}
