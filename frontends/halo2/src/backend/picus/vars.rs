pub use picus::vars::{VarKind, VarStr};

use crate::backend::func::{ArgNo, FieldId, FuncIO};

impl Into<VarStr> for FuncIO {
    fn into(self) -> VarStr {
        match self {
            FuncIO::Arg(arg_no) => arg_no.into(),
            FuncIO::Field(field_id) => field_id.into(),
            FuncIO::Temp(col, row) => format!("temp_{col}_{row}").into(),
        }
    }
}

impl Into<VarStr> for ArgNo {
    fn into(self) -> VarStr {
        format!("input_{self}").into()
    }
}

impl Into<VarStr> for FieldId {
    fn into(self) -> VarStr {
        format!("output_{self}").into()
    }
}

#[derive(Clone, Copy, Hash, Eq, PartialEq)]
pub enum VarKey {
    IO(FuncIO),
    Temp,
    Lifted(FuncIO, usize),
}

impl Default for VarKey {
    fn default() -> Self {
        Self::Temp
    }
}

impl Into<VarStr> for VarKey {
    fn into(self) -> VarStr {
        match self {
            VarKey::IO(func_io) => func_io.into(),
            VarKey::Temp => "temp".to_owned().into(),
            VarKey::Lifted(f, id) => format!(
                "lifted_{}{}",
                match f {
                    FuncIO::Arg(_) => "input_",
                    FuncIO::Field(_) => "output_",
                    FuncIO::Temp(_, _) => "",
                },
                id
            )
            .into(),
        }
    }
}

impl VarKind for VarKey {
    fn is_input(&self) -> bool {
        match self {
            VarKey::IO(func_io) => match func_io {
                FuncIO::Arg(_) => true,
                _ => false,
            },
            VarKey::Lifted(FuncIO::Arg(_), _) => true,
            _ => false,
        }
    }

    fn is_output(&self) -> bool {
        match self {
            VarKey::IO(func_io) => match func_io {
                FuncIO::Field(_) => true,
                _ => false,
            },
            VarKey::Lifted(FuncIO::Field(_), _) => true,
            _ => false,
        }
    }

    fn is_temp(&self) -> bool {
        match self {
            VarKey::IO(func_io) => match func_io {
                FuncIO::Temp(_, _) => true,
                _ => false,
            },
            VarKey::Temp => true,
            _ => false,
        }
    }

    fn temp() -> Self {
        Self::Temp
    }
}

impl<T: Into<FuncIO>> From<T> for VarKey {
    fn from(value: T) -> Self {
        Self::IO(value.into())
    }
}
