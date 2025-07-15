use std::{
    cell::RefCell,
    fmt,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use crate::{
    display::{ListItem, TextRepresentable, TextRepresentation},
    expr::{self, traits::ConstraintEmitter, Expr},
    felt::Felt,
    stmt::{
        self,
        traits::{ConstraintLike as _, StmtConstantFolding as _},
        Stmt,
    },
    vars::{VarAllocator, VarKind, VarStr, Vars},
};

pub type ModuleRef<K> = Rc<RefCell<Module<K>>>;

impl<Key: VarKind + Default + Clone> VarAllocator for ModuleRef<Key> {
    type Kind = Key;

    fn allocate<K: Into<Self::Kind> + Into<VarStr> + Clone>(&self, kind: K) -> VarStr {
        let mut r = self.borrow_mut();
        r.deref_mut().add_var(kind)
    }
}

struct ModuleSummary {
    input_count: usize,
    output_count: usize,
    temp_count: usize,
    constraint_count: usize,
}

type TR<'a> = TextRepresentation<'a>;

#[derive(Clone)]
pub struct ModuleHeader(String);

impl From<String> for ModuleHeader {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Deref for ModuleHeader {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ModuleHeader {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TextRepresentable for ModuleHeader {
    fn to_repr(&self) -> TextRepresentation {
        owned_list!("begin-module", &self.0).break_line()
    }

    fn width_hint(&self) -> usize {
        15 + self.0.width_hint()
    }
}

pub struct Module<K: VarKind> {
    pub(crate) name: ModuleHeader,
    pub(crate) stmts: Vec<Stmt>,
    pub(crate) vars: Vars<K>,
}

impl<K: VarKind + Clone> Clone for Module<K> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            stmts: self.stmts.clone(),
            vars: self.vars.clone(),
        }
    }
}

impl<K: VarKind + Clone> From<ModuleRef<K>> for Module<K> {
    fn from(value: ModuleRef<K>) -> Self {
        value.borrow().clone()
    }
}

impl<K: VarKind + Default> From<String> for Module<K> {
    fn from(name: String) -> Self {
        Self {
            name: name.into(),
            stmts: Default::default(),
            vars: Default::default(),
        }
    }
}

impl<K: VarKind> ConstraintEmitter for Module<K> {
    fn emit(&mut self, lhs: Expr, rhs: Expr) {
        self.stmts.push(stmt::constrain(expr::eq(&lhs, &rhs)))
    }
}

pub trait ModuleLike<K> {
    fn fold_stmts(&mut self, prime: &Felt);

    fn add_constraint(&mut self, constraint: Expr) {
        self.add_stmt(stmt::constrain(constraint))
    }

    fn constraints_len(&self) -> usize;

    fn add_stmt(&mut self, stmt: Stmt);
}

pub trait ModuleWithVars<K> {
    fn add_var<I: Into<K> + Into<VarStr> + Clone>(&mut self, k: I) -> VarStr;

    fn add_vars<I: Into<K> + Into<VarStr> + Clone>(&mut self, it: impl Iterator<Item = I>) {
        it.for_each(|k| {
            self.add_var(k);
        });
    }
}

impl<K: VarKind> ModuleLike<K> for Module<K> {
    fn fold_stmts(&mut self, prime: &Felt) {
        self.stmts = self
            .stmts()
            .iter()
            .map(|s| s.fold(prime).unwrap_or(s.clone()))
            .collect();
    }

    fn constraints_len(&self) -> usize {
        self.stmts.iter().filter(|s| s.is_constraint()).count()
    }

    fn add_stmt(&mut self, stmt: Stmt) {
        self.stmts.push(stmt)
    }
}

impl<K: VarKind> ModuleLike<K> for ModuleRef<K> {
    fn fold_stmts(&mut self, prime: &Felt) {
        self.borrow_mut().fold_stmts(prime)
    }

    fn constraints_len(&self) -> usize {
        self.borrow().constraints_len()
    }

    fn add_stmt(&mut self, stmt: Stmt) {
        self.borrow_mut().add_stmt(stmt)
    }
}

impl<K: VarKind> Module<K> {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn vars(&self) -> &Vars<K> {
        &self.vars
    }

    pub fn stmts(&self) -> &[Stmt] {
        &self.stmts
    }

    pub fn stmts_mut(&mut self) -> &mut [Stmt] {
        &mut self.stmts
    }

    pub fn remove_stmt_if<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Stmt) -> bool,
    {
        self.stmts.retain_mut(|s| !f(s));
    }

    pub fn add_stmts(&mut self, stmts: &[Stmt]) {
        self.stmts.extend_from_slice(stmts)
    }

    pub fn inputs_as_exprs(&self) -> anyhow::Result<Vec<Expr>> {
        self.vars
            .inputs()
            .map(|input| Ok(expr::known_var(&VarStr::try_from(input.to_owned())?)))
            .collect()
    }

    fn summarize(&self) -> ModuleSummary {
        let input_count = self.vars.inputs().count();
        let output_count = self.vars.outputs().count();
        let temp_count = self.vars.temporaries().count();
        let constraint_count = self.stmts.iter().filter(|s| s.is_constraint()).count();

        ModuleSummary {
            input_count,
            output_count,
            temp_count,
            constraint_count,
        }
    }
}

impl<K: VarKind + Default + Clone + fmt::Debug> Module<K> {
    pub fn new<S: Into<K> + Into<VarStr> + Clone>(
        name: String,
        inputs: impl Iterator<Item = S>,
        outputs: impl Iterator<Item = S>,
    ) -> Self {
        let mut m = Self::from(name);
        for k in inputs.chain(outputs) {
            m.add_var(k);
        }
        m
    }
    pub fn shared<S: Into<K> + Into<VarStr> + Clone>(
        name: String,
        inputs: impl Iterator<Item = S>,
        outputs: impl Iterator<Item = S>,
    ) -> ModuleRef<K> {
        Rc::new(Self::new(name, inputs, outputs).into())
    }
}

impl<K: VarKind + Default + Clone + fmt::Debug> ModuleWithVars<K> for Module<K> {
    fn add_var<I: Into<K> + Into<VarStr> + Clone>(&mut self, k: I) -> VarStr {
        self.vars.insert(k)
    }

    fn add_vars<I: Into<K> + Into<VarStr> + Clone>(&mut self, it: impl Iterator<Item = I>) {
        self.vars.extend(it)
    }
}

impl<K: VarKind> TextRepresentable for Module<K> {
    fn to_repr(&self) -> TextRepresentation {
        let summary = self.summarize();
        let mut sorted_inputs = self.vars.inputs().collect::<Vec<_>>();
        sorted_inputs.sort();
        let mut sorted_outputs = self.vars.outputs().collect::<Vec<_>>();
        sorted_outputs.sort();
        owned_list!(&self.name)
            + [
                format!("Number of inputs:      {}", summary.input_count),
                format!("Number of outputs:     {}", summary.output_count),
                format!("Number of temporaries: {}", summary.temp_count),
                format!("Number of constraints: {}", summary.constraint_count),
            ]
            .into_iter()
            .map(TR::owned_comment)
            .sum()
            + TR::owned_list(
                &sorted_inputs
                    .into_iter()
                    .map(|i: &str| owned_list!("input", i).break_line().into())
                    .collect::<Vec<ListItem>>(),
            )
            + TR::owned_list(
                &sorted_outputs
                    .into_iter()
                    .map(|o: &str| owned_list!("output", o).break_line().into())
                    .collect::<Vec<ListItem>>(),
            )
            + self.stmts.to_repr()
            + owned_list!(owned_list!("end-module"))
            + TR::comment(self.name())
    }

    fn width_hint(&self) -> usize {
        self.name.width_hint()
    }
}
