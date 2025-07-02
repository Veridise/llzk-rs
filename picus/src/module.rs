use std::{cell::RefCell, fmt, ops::DerefMut as _, rc::Rc};

use crate::{
    expr::Expr,
    stmt::{self, Stmt},
    vars::{VarAllocator, VarKind, VarStr, Vars},
};

pub type ModuleRef<K> = Rc<RefCell<Module<K>>>;

impl<Key: VarKind + Into<VarStr> + Default + Clone> VarAllocator for ModuleRef<Key> {
    type Kind = Key;

    fn allocate<K: Into<Self::Kind>>(&self, kind: K) -> VarStr {
        let mut r = self.borrow_mut();
        r.deref_mut().add_var(kind.into())
    }
}

struct ModuleSummary {
    input_count: usize,
    output_count: usize,
    temp_count: usize,
    constraint_count: usize,
}

impl fmt::Display for ModuleSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "; Number of inputs:      {}", self.input_count)?;
        writeln!(f, "; Number of outputs:     {}", self.output_count)?;
        writeln!(f, "; Number of temporaries: {}", self.temp_count)?;
        writeln!(f, "; Number of constraints: {}", self.constraint_count)
    }
}

pub struct Module<K: VarKind> {
    pub(crate) name: String,
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
            name,
            stmts: Default::default(),
            vars: Default::default(),
        }
    }
}

pub trait ModuleLike<K> {
    fn fold_stmts(&mut self);

    fn add_constraint(&mut self, constraint: Expr) {
        self.add_stmt(stmt::constrain(constraint))
    }

    fn constraints_len(&self) -> usize;

    fn add_stmt(&mut self, stmt: Stmt);
}

pub trait ModuleWithVars<K> {
    fn add_var<I: Into<K>>(&mut self, k: I) -> VarStr;
}

impl<K: VarKind> ModuleLike<K> for Module<K> {
    fn fold_stmts(&mut self) {
        self.stmts = self
            .stmts()
            .iter()
            .map(|s| s.fold().unwrap_or(s.clone()))
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
    fn fold_stmts(&mut self) {
        self.borrow_mut().fold_stmts()
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

    pub fn stmts(&self) -> &[Stmt] {
        &self.stmts
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

impl<K: VarKind + Default + Into<VarStr> + Clone> Module<K> {
    pub fn new<I: Into<K>, O: Into<K>>(
        name: String,
        inputs: impl Iterator<Item = I>,
        outputs: impl Iterator<Item = O>,
    ) -> Self {
        let mut m = Self::from(name);
        for k in inputs.map(Into::into).chain(outputs.map(Into::into)) {
            m.add_var(k);
        }
        m
    }
    pub fn shared<I: Into<K>, O: Into<K>>(
        name: String,
        inputs: impl Iterator<Item = I>,
        outputs: impl Iterator<Item = O>,
    ) -> ModuleRef<K> {
        Rc::new(Self::new(name, inputs, outputs).into())
    }
}

impl<K: VarKind + Default + Into<VarStr> + Clone> ModuleWithVars<K> for Module<K> {
    fn add_var<I: Into<K>>(&mut self, k: I) -> VarStr {
        self.vars.insert(k.into())
    }
}

impl<K: VarKind> fmt::Display for Module<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "(begin-module {})", self.name)?;
        writeln!(f, "{}", self.summarize())?;
        for i in self.vars.inputs() {
            writeln!(f, "(input {i})")?;
        }
        for o in self.vars.outputs() {
            writeln!(f, "(output {o})")?;
        }
        for c in &self.stmts {
            write!(f, "{c}")?;
        }
        writeln!(f, "(end-module)")
    }
}
