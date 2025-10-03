use log::Log;
use melior::diagnostic::{Diagnostic, DiagnosticSeverity};

fn log_msg(diag: &Diagnostic, logger: &dyn Log) {
    match diag.severity() {
        DiagnosticSeverity::Error => {
            log::error!(logger: logger, "[{}] {}", diag.location(), diag.to_string())
        }
        DiagnosticSeverity::Note => {
            log::info!(logger: logger, "[{}] note: {}", diag.location(), diag.to_string())
        }
        DiagnosticSeverity::Remark => {
            log::info!(logger: logger, "[{}] remark: {}", diag.location(), diag.to_string())
        }
        DiagnosticSeverity::Warning => {
            log::warn!(logger: logger, "[{}] {}", diag.location(), diag.to_string())
        }
    }
}

fn log_notes(diag: &Diagnostic, logger: &dyn Log) -> Result<(), bool> {
    for note_no in 0..diag.note_count() {
        let note = diag.note(note_no);
        match note {
            Ok(note) => {
                log_msg(&note, logger);
                log_notes(&note, logger)?;
            }
            Err(err) => {
                log::error!(logger: logger, "Error while obtaining note #{note_no}: {err}");
                return Err(false);
            }
        };
    }
    Ok(())
}

/// Diagnostics handler that writes the diagnostics to the [`log`].
pub fn log_diagnostic(diag: Diagnostic, logger: &dyn Log) -> bool {
    log_msg(&diag, logger);
    if let Err(res) = log_notes(&diag, logger) {
        return res;
    }

    match diag.severity() {
        DiagnosticSeverity::Error => false,
        _ => true,
    }
}
