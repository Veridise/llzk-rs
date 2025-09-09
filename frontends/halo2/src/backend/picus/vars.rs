use std::borrow::Cow;

use picus::vars::Temp;
pub use picus::vars::{VarKind, VarStr};

use crate::{backend::func::FuncIO, synthesis::regions::FQN};

fn prepend_fqn(fqn: Option<Cow<FQN>>) -> String {
    match fqn {
        Some(fqn) => format!("{fqn}__"),
        None => "".to_string(),
    }
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub enum VarKeySeedInner<'a> {
    IO(FuncIO, Option<Cow<'a, FQN>>),
    Temp,
    Lifted(usize),
}

impl VarKeySeed<'_> {
    pub fn arg(arg_no: usize, conv: NamingConvention) -> Self {
        Self(VarKeySeedInner::IO(FuncIO::Arg(arg_no.into()), None), conv)
    }

    pub fn field(field_no: usize, conv: NamingConvention) -> Self {
        Self(
            VarKeySeedInner::IO(FuncIO::Field(field_no.into()), None),
            conv,
        )
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
            VarKey::IO(func_io) => matches!(func_io, FuncIO::Advice(_)),
            VarKey::Temp => true,
            VarKey::Lifted(_) => false,
        }
    }
}

impl Default for VarKeySeedInner<'_> {
    fn default() -> Self {
        Self::Temp
    }
}

impl Default for VarKey {
    fn default() -> Self {
        Self::Temp
    }
}

impl<'a> Temp<'a> for VarKey {
    type Ctx = NamingConvention;
    type Output = VarKeySeed<'a>;

    fn temp(conv: Self::Ctx) -> Self::Output {
        VarKeySeed(VarKeySeedInner::Temp, conv)
    }
}

#[derive(Clone, Copy)]
pub enum NamingConvention {
    Default,
    Short,
}

impl NamingConvention {
    fn format_io(&self, func_io: FuncIO, fqn: Option<Cow<FQN>>) -> String {
        match self {
            NamingConvention::Default => format!(
                "{}{}",
                prepend_fqn(fqn),
                match func_io {
                    FuncIO::Arg(arg_no) => format!("Input_{arg_no}"),
                    FuncIO::Field(field_id) => format!("Output_{field_id}"),
                    FuncIO::Advice(adv) => format!("Advice_{}_{}", adv.col(), adv.row()),
                    FuncIO::Fixed(fix) => format!("Fixed_{}_{}", fix.col(), fix.row()),
                    FuncIO::TableLookup(id, col, row, idx, ridx) =>
                        format!("Lookup_{id}_{col}_{row}_{idx}_{ridx}"),
                    FuncIO::CallOutput(module, out) => format!("CallOutput_{module}_{out}"),
                }
            ),
            NamingConvention::Short => match func_io {
                FuncIO::Arg(arg_no) => format!("in_{arg_no}"),
                FuncIO::Field(field_id) => format!("out_{field_id}"),
                FuncIO::Advice(adv) => format!("adv_{}_{}", adv.col(), adv.row()),
                FuncIO::Fixed(fix) => format!("fix_{}_{}", fix.col(), fix.row()),
                FuncIO::TableLookup(id, col, row, idx, ridx) => {
                    format!("lkp{id}_{col}_{row}_{idx}_{ridx}")
                }
                FuncIO::CallOutput(module, out) => format!("cout_{module}_{out}"),
            },
        }
    }

    fn format_temp(&self) -> String {
        match self {
            NamingConvention::Default => "Temp_",
            NamingConvention::Short => "t",
        }
        .to_owned()
    }

    fn format_lifted(&self, id: usize) -> String {
        match self {
            NamingConvention::Default => format!("Lifted_Input_{id}"),
            NamingConvention::Short => format!("l{id}"),
        }
    }
}

#[derive(Clone)]
pub struct VarKeySeed<'a>(VarKeySeedInner<'a>, NamingConvention);

impl<'a> VarKeySeed<'a> {
    pub fn new(inner: VarKeySeedInner<'a>, conv: NamingConvention) -> Self {
        Self(inner, conv)
    }

    pub fn io<I: Into<FuncIO>>(i: I, conv: NamingConvention) -> Self {
        Self(VarKeySeedInner::IO(i.into(), None), conv)
    }

    pub fn named_io<I: Into<FuncIO>>(
        i: I,
        fqn: Option<Cow<'a, FQN>>,
        conv: NamingConvention,
    ) -> Self {
        Self(VarKeySeedInner::IO(i.into(), fqn), conv)
    }

    pub fn lifted(id: usize, conv: NamingConvention) -> Self {
        Self(VarKeySeedInner::Lifted(id), conv)
    }
}

impl From<VarKeySeed<'_>> for VarKey {
    fn from(seed: VarKeySeed) -> VarKey {
        match seed.0 {
            VarKeySeedInner::IO(func_io, _) => VarKey::IO(func_io),
            VarKeySeedInner::Temp => VarKey::Temp,
            VarKeySeedInner::Lifted(idx) => VarKey::Lifted(idx),
        }
    }
}

impl From<VarKeySeed<'_>> for VarStr {
    fn from(seed: VarKeySeed) -> VarStr {
        match seed.0 {
            VarKeySeedInner::IO(func_io, fqn) => seed.1.format_io(func_io, fqn),
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
            VarKey::IO(func_io) => matches!(func_io, FuncIO::Advice(_)),
            VarKey::Temp => true,
            _ => false,
        }
    }
}

impl<'a> From<(FuncIO, Option<Cow<'a, FQN>>)> for VarKeySeedInner<'a> {
    fn from(value: (FuncIO, Option<Cow<'a, FQN>>)) -> Self {
        Self::IO(value.0, value.1)
    }
}

impl<T: Into<FuncIO>> From<T> for VarKeySeedInner<'_> {
    fn from(value: T) -> Self {
        Self::IO(value.into(), None)
    }
}
