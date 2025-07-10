use picus::vars::Temp;
pub use picus::vars::{VarKind, VarStr};

use crate::{
    backend::func::{ArgNo, FuncIO},
    synthesis::regions::FQN,
};

fn prepend_fqn(fqn: Option<FQN>) -> String {
    match fqn {
        Some(fqn) => format!("{fqn}__"),
        None => "".to_string(),
    }
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub enum VarKeySeed {
    IO(FuncIO, Option<FQN>),
    Temp,
    Lifted(usize),
}

impl VarKeySeed {
    pub fn arg(arg_no: usize) -> Self {
        Self::IO(FuncIO::Arg(arg_no.into()), None)
    }

    pub fn field(field_no: usize) -> Self {
        Self::IO(FuncIO::Field(field_no.into()), None)
    }
}

#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug)]
pub enum VarKey {
    IO(FuncIO),
    Temp,
    Lifted(usize),
}

impl VarKey {
    pub fn is_temp(&self) -> bool {
        match self {
            VarKey::IO(func_io) => matches!(func_io, FuncIO::Advice(_, _)),
            VarKey::Temp => true,
            VarKey::Lifted(_) => false,
        }
    }
}

impl Default for VarKeySeed {
    fn default() -> Self {
        Self::Temp
    }
}

impl Default for VarKey {
    fn default() -> Self {
        Self::Temp
    }
}

impl Temp for VarKey {
    type Output = VarKeySeed;

    fn temp() -> Self::Output {
        VarKeySeed::Temp
    }
}

impl From<VarKeySeed> for VarKey {
    fn from(seed: VarKeySeed) -> VarKey {
        match seed {
            VarKeySeed::IO(func_io, _) => VarKey::IO(func_io),
            VarKeySeed::Temp => VarKey::Temp,
            VarKeySeed::Lifted(idx) => VarKey::Lifted(idx),
        }
    }
}
impl From<VarKeySeed> for VarStr {
    fn from(key: VarKeySeed) -> VarStr {
        match key {
            VarKeySeed::IO(func_io, fqn) => format!(
                "{}{}",
                prepend_fqn(fqn),
                match func_io {
                    FuncIO::Arg(arg_no) => format!("Input_{arg_no}"),
                    FuncIO::Field(field_id) => format!("Output_{field_id}"),
                    FuncIO::Advice(col, row) => format!("Advice_{col}_{row}"),
                }
            )
            .try_into()
            .unwrap(),
            VarKeySeed::Temp => "Temp_".to_owned().try_into().unwrap(),
            VarKeySeed::Lifted(id) => format!("Lifted_Input_{}", id).try_into().unwrap(),
        }
    }
}

impl VarKind for VarKey {
    fn is_input(&self) -> bool {
        match self {
            VarKey::IO(func_io) => matches!(func_io, FuncIO::Arg(_)),
            VarKey::Lifted(_) => true,
            _ => false,
        }
    }

    fn is_output(&self) -> bool {
        match self {
            VarKey::IO(func_io) => matches!(func_io, FuncIO::Field(_)),
            _ => false,
        }
    }

    fn is_temp(&self) -> bool {
        match self {
            VarKey::IO(func_io) => matches!(func_io, FuncIO::Advice(_, _)),
            VarKey::Temp => true,
            _ => false,
        }
    }
}

impl From<(FuncIO, Option<FQN>)> for VarKeySeed {
    fn from(value: (FuncIO, Option<FQN>)) -> Self {
        Self::IO(value.0, value.1)
    }
}

impl<T: Into<FuncIO>> From<T> for VarKeySeed {
    fn from(value: T) -> Self {
        Self::IO(value.into(), None)
    }
}
