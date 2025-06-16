use std::fmt;

use crate::backend::func::{ArgNo, FieldId, FuncIO};

#[derive(Clone)]
pub struct VarStr(String);

impl From<String> for VarStr {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<VarKey> for VarStr {
    fn from(value: VarKey) -> Self {
        match value {
            VarKey::IO(func_io) => func_io.into(),
            VarKey::Temp(n) => format!("temp_{n}").into(),
        }
    }
}

impl From<FuncIO> for VarStr {
    fn from(value: FuncIO) -> Self {
        match value {
            FuncIO::Arg(arg_no) => arg_no.into(),
            FuncIO::Field(field_id) => field_id.into(),
            FuncIO::Temp(col, row) => format!("temp_{col}_{row}").into(),
        }
    }
}

impl From<ArgNo> for VarStr {
    fn from(value: ArgNo) -> Self {
        format!("input_{value}").into()
    }
}

impl From<FieldId> for VarStr {
    fn from(value: FieldId) -> Self {
        format!("output_{value}").into()
    }
}

impl fmt::Display for VarStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub trait VarAllocator {
    type Kind;

    fn allocate<K: Into<Self::Kind>>(&self, kind: K) -> VarStr;

    fn allocate_temp(&self) -> VarStr;
}

pub trait VarIO {
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
