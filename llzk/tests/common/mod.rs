use log::LevelFilter;
use melior::diagnostic::{Diagnostic, DiagnosticSeverity};
use simplelog::{Config, TestLogger};

pub fn setup() {
    let _ = TestLogger::init(LevelFilter::Debug, Config::default());
}
