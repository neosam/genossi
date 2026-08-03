//! Regressionstest fuer den Dioxus form-onsubmit / Page-Reload-Bug.
//!
//! Diese Codebase hat gelernt: `form { onsubmit: e.prevent_default() }` verhindert
//! den Page-Reload in Dioxus NICHT zuverlaessig. Der etablierte Fix ist
//! `div`-Wrapper + `r#type:"button"` + `onclick` (Referenz: `page/repayment_phases.rs`,
//! `CreateRepaymentPhaseForm`).
//!
//! Ein Button mit `r#type: "submit"` innerhalb eines `<form>` (oder ein `<form>` mit
//! `onsubmit`) loest beim Klick / Enter einen ungewollten Full-Page-Reload aus. Weil
//! `prevent_default()` das nicht zuverlaessig stoppt, ist die Regel in diesem Frontend:
//! KEINE Submit-Buttons — Submit-Logik gehoert in einen `onclick`-Handler auf einem
//! `r#type:"button"`-Button.
//!
//! Dieser Test scannt den kompletten Frontend-Quellcode und schlaegt fehl, sobald das
//! Anti-Pattern wieder eingefuehrt wird.

use std::fs;
use std::path::{Path, PathBuf};

/// Rekursiv alle `.rs`-Dateien unterhalb von `dir` einsammeln.
fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("src-Verzeichnis lesbar") {
        let path = entry.expect("DirEntry lesbar").path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn no_submit_type_buttons_in_frontend_source() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs_files(&src, &mut files);
    assert!(
        !files.is_empty(),
        "keine Rust-Quelldateien unter src/ gefunden — Test-Setup fehlerhaft"
    );

    let mut offenders = Vec::new();
    for file in &files {
        let content = fs::read_to_string(file).expect("Quelldatei lesbar");
        for (idx, line) in content.lines().enumerate() {
            // Exakte Attribut-Schreibweise; taucht in keinem Kommentar auf.
            if line.contains(r#"r#type: "submit""#) {
                offenders.push(format!("{}:{}", file.display(), idx + 1));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "Submit-Buttons gefunden — diese loesen in Dioxus einen ungewollten \
         Page-Reload aus. Nutze `r#type: \"button\"` + `onclick` statt \
         `<form onsubmit>` / `r#type: \"submit\"` (Referenz: \
         page/repayment_phases.rs):\n{}",
        offenders.join("\n")
    );
}
