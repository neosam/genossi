# Deferred Items — Quick 260908-cjo

## genossi-frontend/Cargo.lock haengt eine Version hinterher

`genossi-frontend/Cargo.lock` fuehrt `genossi-frontend` weiterhin mit
`2026.233.2-dev`, waehrend `genossi-frontend/Cargo.toml` seit Commit `7bbd18e`
("Set version to 2026.251.1-dev") auf `2026.251.1-dev` steht. Der
Version-Bump-Vorgang aktualisiert offenbar nur den Root-Lock, nicht den des
aus dem Workspace exkludierten Frontend-Crates.

Sichtbar wurde das, weil `cargo test --manifest-path genossi-frontend/Cargo.toml`
den Lock beim ersten Lauf mechanisch nachzieht und den Arbeitsbaum dirty
hinterlaesst.

**Nicht von diesem Quick verursacht** und deshalb bewusst nicht mitcommittet
(Scope-Boundary: nur Folgen der eigenen Aenderungen werden repariert). Gehoert
in die `release-version`-Skill-/Bump-Logik, nicht hierher.
