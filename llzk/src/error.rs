use std::{
    convert::Infallible,
    error,
    fmt::{self, Display, Formatter},
    str::Utf8Error,
};

use melior::diagnostic::{Diagnostic, DiagnosticSeverity};

type MeliorError = melior::Error;

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    BuildMthdFailed(&'static str),
    OutOfBoundsArgument(Option<String>, usize),
    OperationExpected(&'static str, String),
    BlockExpected(usize),
    EmptyBlock,
    Melior(MeliorError),
    AttributeNotFound(String),
    Diagnostics(DiagnosticErrors),
    OpVerificationFailed {
        name: String,
        ir: String,
        location: String,
        diags: Option<DiagnosticErrors>,
    },
}

impl error::Error for Error {}

impl From<Utf8Error> for Error {
    fn from(error: Utf8Error) -> Self {
        Self::Melior(MeliorError::Utf8(error))
    }
}

impl From<Infallible> for Error {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

impl From<MeliorError> for Error {
    fn from(value: MeliorError) -> Self {
        Self::Melior(value)
    }
}

impl From<DiagnosticError> for Error {
    fn from(value: DiagnosticError) -> Self {
        Self::Diagnostics(DiagnosticErrors(vec![value]))
    }
}

impl From<Vec<DiagnosticError>> for Error {
    fn from(value: Vec<DiagnosticError>) -> Self {
        Self::Diagnostics(DiagnosticErrors(value))
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Error::OperationExpected(op, actual) => write!(f, "{op} op expected: {actual}"),
            Error::Melior(error) => Display::fmt(error, f),
            Error::OutOfBoundsArgument(func_name, index) => {
                write!(f, "index {index} out of bounds ")?;
                match func_name {
                    Some(func_name) => {
                        write!(f, "function {func_name}")
                    }
                    None => write!(f, "block"),
                }
            }
            Error::BuildMthdFailed(mthd) => write!(f, "build method '{mthd}' failed"),
            Error::BlockExpected(nth) => {
                write!(
                    f,
                    "region was expected to have at least {} block{}",
                    nth + 1,
                    if *nth == 0 { "" } else { "s" }
                )
            }
            Error::EmptyBlock => write!(f, "block was expected not to be empty"),
            Error::AttributeNotFound(attr) => write!(f, "attribute was not found: {attr}"),
            Error::Diagnostics(diagnostics) => Display::fmt(diagnostics, f),
            Error::OpVerificationFailed {
                name,
                ir,
                location,
                diags,
            } => {
                write!(f, "{location}: '{name}' op verification failed")?;
                if let Some(diags) = diags {
                    write!(f, ": ")?;
                    Display::fmt(diags, f)?;
                }
                if f.alternate() {
                    writeln!(f, "  {ir}")?;
                }
                Ok(())
            }
        }
    }
}

/// Represents a diagnostic emitted by MLIR level.
///
/// Stores the text representation of the diagnostic and is not linked to the lifetime of the
/// diagnostics engine or MLIR objects in general.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DiagnosticError {
    severity: u32,
    location: String,
    msg: String,
    notes: DiagnosticErrors,
}

impl DiagnosticError {
    /// Returns the severity of the diagnostic, if valid.
    pub fn severity(&self) -> Option<DiagnosticSeverity> {
        DiagnosticSeverity::try_from(self.severity).ok()
    }

    fn fmt_severity(&self) -> &'static str {
        let Some(severity) = self.severity() else {
            return "";
        };
        match severity {
            DiagnosticSeverity::Error => " error:",
            DiagnosticSeverity::Note => " note:",
            DiagnosticSeverity::Remark => " remark:",
            DiagnosticSeverity::Warning => " warning:",
        }
    }
}

impl From<Diagnostic<'_>> for DiagnosticError {
    fn from(diag: Diagnostic<'_>) -> Self {
        #[allow(non_upper_case_globals)]
        let severity = match diag.severity() {
            DiagnosticSeverity::Error => mlir_sys::MlirDiagnosticSeverity_MlirDiagnosticError,
            DiagnosticSeverity::Note => mlir_sys::MlirDiagnosticSeverity_MlirDiagnosticNote,
            DiagnosticSeverity::Remark => mlir_sys::MlirDiagnosticSeverity_MlirDiagnosticRemark,
            DiagnosticSeverity::Warning => mlir_sys::MlirDiagnosticSeverity_MlirDiagnosticWarning,
        };
        let location = diag.location().to_string();
        let msg = diag.to_string();
        let notes = DiagnosticErrors(
            (0..diag.note_count())
                .map(|i| Self::from(diag.note(i).unwrap()))
                .collect(),
        );
        Self {
            severity,
            location,
            msg,
            notes,
        }
    }
}

impl error::Error for DiagnosticError {}

impl Display for DiagnosticError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{} {}", self.location, self.fmt_severity(), self.msg)?;
        if f.alternate() {
            Display::fmt(&self.notes, f)?;
        }
        Ok(())
    }
}

/// Collection of [`DiagnosticError`].
#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub struct DiagnosticErrors(pub(crate) Vec<DiagnosticError>);

impl error::Error for DiagnosticErrors {}

impl Display for DiagnosticErrors {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for diag in &self.0 {
            writeln!(f)?;
            Display::fmt(diag, f)?;
            writeln!(f)?;
        }
        Ok(())
    }
}

impl<I: Into<DiagnosticError>> FromIterator<I> for DiagnosticErrors {
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
        Self(Vec::from_iter(iter.into_iter().map(Into::into)))
    }
}

impl<I: Into<DiagnosticError>> Extend<I> for DiagnosticErrors {
    fn extend<T: IntoIterator<Item = I>>(&mut self, iter: T) {
        self.0.extend(iter.into_iter().map(Into::into))
    }
}
