use picus::vars::Temp;
pub use picus::vars::{VarKind, VarStr};

use haloumi_ir_base::func::FuncIO;

/// Inner value of [`VarKeySeed`].
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub enum VarKeySeedInner {
    IO(FuncIO),
    Temp,
    Lifted(usize),
}

impl VarKeySeed {
    pub fn arg(arg_no: usize, conv: NamingConvention) -> Self {
        Self(VarKeySeedInner::IO(FuncIO::Arg(arg_no.into())), conv)
    }

    pub fn field(field_no: usize, conv: NamingConvention) -> Self {
        Self(VarKeySeedInner::IO(FuncIO::Field(field_no.into())), conv)
    }
}

#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug)]
pub enum VarKey {
    IO(FuncIO),
    Temp,
    Lifted(usize),
}

impl Default for VarKeySeedInner {
    fn default() -> Self {
        Self::Temp
    }
}

impl Default for VarKey {
    fn default() -> Self {
        Self::Temp
    }
}

impl Temp<'_> for VarKey {
    type Ctx = NamingConvention;
    type Output = VarKeySeed;

    fn temp(conv: Self::Ctx) -> Self::Output {
        VarKeySeed(VarKeySeedInner::Temp, conv)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum NamingConvention {
    Short,
}

impl NamingConvention {
    fn format_io(&self, func_io: FuncIO) -> String {
        match self {
            NamingConvention::Short => match func_io {
                FuncIO::Arg(arg_no) => format!("in_{arg_no}"),
                FuncIO::Field(field_id) => format!("out_{field_id}"),
                FuncIO::Advice(adv) => format!("adv_{}_{}", adv.col(), adv.row()),
                FuncIO::Fixed(fix) => format!("fix_{}_{}", fix.col(), fix.row()),
                FuncIO::TableLookup(id, col, row, idx, ridx) => {
                    format!("lkp{id}_{col}_{row}_{idx}_{ridx}")
                }
                FuncIO::CallOutput(module, out) => format!("cout_{module}_{out}"),
                FuncIO::Temp(temp) => format!("t{}", temp),
                FuncIO::Challenge(index, phase, _) => format!("chall_{index}_{phase}"),
            },
        }
    }

    fn format_temp(&self) -> String {
        match self {
            // These temps are exclusive from the Picus backend so we use 'pt' for 'Picus temp'.
            NamingConvention::Short => "pt",
        }
        .to_owned()
    }

    fn format_lifted(&self, id: usize) -> String {
        match self {
            NamingConvention::Short => format!("l{id}"),
        }
    }
}

/// Struct containing the metadata necessary to create a [`VarStr`].
#[derive(Clone, Debug)]
pub struct VarKeySeed(VarKeySeedInner, NamingConvention);

impl VarKeySeed {
    pub fn new(inner: VarKeySeedInner, conv: NamingConvention) -> Self {
        Self(inner, conv)
    }

    pub fn io<I: Into<FuncIO>>(i: I, conv: NamingConvention) -> Self {
        Self(VarKeySeedInner::IO(i.into()), conv)
    }

    pub fn lifted(id: usize, conv: NamingConvention) -> Self {
        Self(VarKeySeedInner::Lifted(id), conv)
    }
}

impl From<VarKeySeed> for VarKey {
    fn from(seed: VarKeySeed) -> VarKey {
        match seed.0 {
            VarKeySeedInner::IO(func_io) => VarKey::IO(func_io),
            VarKeySeedInner::Temp => VarKey::Temp,
            VarKeySeedInner::Lifted(idx) => VarKey::Lifted(idx),
        }
    }
}

impl From<VarKeySeed> for VarStr {
    fn from(seed: VarKeySeed) -> VarStr {
        match seed.0 {
            VarKeySeedInner::IO(func_io) => seed.1.format_io(func_io),
            VarKeySeedInner::Temp => seed.1.format_temp(),
            VarKeySeedInner::Lifted(id) => seed.1.format_lifted(id),
        }
        .try_into()
        .unwrap()
    }
}

impl VarKind for VarKey {
    fn is_input(&self) -> bool {
        match self {
            VarKey::IO(func_io) => matches!(func_io, FuncIO::Arg(_) | FuncIO::Challenge(_, _, _)),
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
            VarKey::IO(func_io) => matches!(
                func_io,
                FuncIO::Advice(_) | FuncIO::CallOutput(_, _) | FuncIO::Temp(_)
            ),
            VarKey::Temp => true,
            _ => false,
        }
    }

    fn get_input_no(&self) -> Option<usize> {
        match self {
            VarKey::IO(FuncIO::Arg(n)) => Some(**n),
            VarKey::IO(FuncIO::Challenge(_, _, n)) => Some(**n),
            _ => None,
        }
    }

    fn get_output_no(&self) -> Option<usize> {
        match self {
            VarKey::IO(FuncIO::Field(n)) => Some(**n),
            _ => None,
        }
    }
}

impl<T: Into<FuncIO>> From<T> for VarKeySeedInner {
    fn from(value: T) -> Self {
        Self::IO(value.into())
    }
}
