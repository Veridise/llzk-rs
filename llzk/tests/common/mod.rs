use log::LevelFilter;
use melior::diagnostic::{Diagnostic, DiagnosticSeverity};
use simplelog::{Config, TestLogger};

pub fn setup() {
    let _ = TestLogger::init(LevelFilter::Debug, Config::default());
}

/// Diagnostics handler that writes the diagnostics to the [`log`].
pub fn diag_logger(diag: Diagnostic) -> bool {
    fn log_msg(diag: &Diagnostic) {
        match diag.severity() {
            DiagnosticSeverity::Error => {
                log::error!("[{}] {}", diag.location(), diag.to_string())
            }
            DiagnosticSeverity::Note => {
                log::info!("[{}] note: {}", diag.location(), diag.to_string())
            }
            DiagnosticSeverity::Remark => {
                log::info!("[{}] remark: {}", diag.location(), diag.to_string())
            }
            DiagnosticSeverity::Warning => log::warn!("[{}] {}", diag.location(), diag.to_string()),
        }
    }
    fn log_notes(diag: &Diagnostic) -> Result<(), bool> {
        for note_no in 0..diag.note_count() {
            let note = diag.note(note_no);
            match note {
                Ok(note) => {
                    log_msg(&note);
                    log_notes(&note)?;
                }
                Err(err) => {
                    log::error!("Error while obtaining note #{note_no}: {err}");
                    return Err(false);
                }
            };
        }
        Ok(())
    }
    log_msg(&diag);
    if let Err(res) = log_notes(&diag) {
        return res;
    }

    match diag.severity() {
        DiagnosticSeverity::Error => false,
        _ => true,
    }
}
