## Context

Die aktuelle `TemplateWorld`-Implementierung in `genossi_service_impl/src/pdf_generation.rs` implementiert das Typst `World`-Trait und löst Dateien ausschließlich relativ zum `template_base`-Verzeichnis auf. Wenn ein Template `#import "@preview/letter-pro:3.0.0"` enthält, erzeugt Typst eine `FileId` mit einer `PackageSpec` — diese wird aktuell ignoriert und führt zu einem "File not found"-Fehler.

Typst-Packages werden als `.tar.gz`-Archive unter `https://packages.typst.org/{namespace}/{name}-{version}.tar.gz` bereitgestellt.

## Goals / Non-Goals

**Goals:**
- Typst-Package-Imports (`@namespace/name:version`) funktionieren transparent in Templates
- Packages werden beim ersten Zugriff automatisch heruntergeladen und lokal gecached
- Verhalten soll dem der Typst CLI entsprechen (exakte Versionen, persistenter Cache)

**Non-Goals:**
- Kein Package-Management-UI (kein Installieren/Deinstallieren über das Frontend)
- Kein Support für lokale/custom Package-Registries
- Kein Versionierungs-Wildcard-Support (nur exakte Versionen wie `3.0.0`)
- Kein Cache-Eviction oder Größenlimit

## Decisions

### Package-Cache als eigene Struct

Der Package-Download und -Cache wird in eine eigene `PackageCache`-Struct ausgelagert, nicht inline in `TemplateWorld`.

**Warum:** `TemplateWorld` wird pro Render-Aufruf neu erstellt, aber der Cache soll persistent sein und zwischen Render-Aufrufen geteilt werden. `PackageCache` wird in `PdfGenerator` gehalten und per Referenz an `TemplateWorld` übergeben.

**Alternative:** Alles inline in `TemplateWorld` → Cache würde bei jedem Render verloren gehen oder müsste als `Arc<Mutex<...>>` nach oben gereicht werden.

### Cache-Verzeichnisstruktur

```
{cache_dir}/
  preview/
    letter-pro/
      3.0.0/
        typst.toml
        lib.typ
        ...
```

Folgt dem Typst-Standard-Layout. `cache_dir` wird über `TYPST_PACKAGE_CACHE` konfiguriert, Default: `./typst-packages`.

**Warum:** Kompatibel mit dem Layout, das Typst CLI verwendet. Einfach debuggbar (man kann ins Verzeichnis schauen).

### Synchrones Herunterladen im `World`-Trait

Das `World`-Trait ist synchron (`source()` und `file()` geben `FileResult<T>` zurück, kein Future). Der Download muss daher blockierend erfolgen.

**Warum:** `typst::compile()` ist synchron. Wir verwenden `reqwest::blocking::Client` für den Download innerhalb der `source()`/`file()`-Methode. Da der Download nur beim allerersten Zugriff auf ein Package stattfindet, ist die Blockierung akzeptabel.

**Alternative:** Vorher asynchron downloaden, bevor `compile()` aufgerufen wird → erfordert vorheriges Parsen des Templates um Imports zu erkennen, deutlich komplexer.

### Pfadauflösung in `resolve_path()`

```rust
fn resolve_path(&self, id: FileId) -> PathBuf {
    if let Some(pkg) = id.package() {
        self.package_cache.package_dir(pkg).join(id.vpath().as_rootless_path())
    } else {
        self.template_base.join(id.vpath().as_rootless_path())
    }
}
```

`source()` und `file()` rufen bei Package-FileIds zusätzlich `self.package_cache.ensure_downloaded(pkg)` auf, bevor sie die Datei lesen.

### Fehlerbehandlung

- **Download-Fehler** (Netzwerk, 404): Wird als `typst::diag::FileError::Other` zurückgegeben → Typst erzeugt eine Fehlermeldung, die als `TemplateError::RenderError` an den API-Client weitergereicht wird
- **Ungültiges Archiv**: Gleiche Behandlung wie Download-Fehler
- **Package existiert nicht im Registry**: HTTP 404 → sinnvolle Fehlermeldung "Package @preview/foo:1.0.0 not found"

### Dependencies

- `reqwest` mit `blocking`-Feature: Bereits als Workspace-Dependency vorhanden (für Tests), muss für `genossi_service_impl` als Dependency hinzugefügt werden
- `flate2`: Gzip-Dekompression
- `tar`: Tar-Archiv-Entpackung

## Risks / Trade-offs

- **Erster Render mit neuem Package ist langsam** → Akzeptabel, da einmalig pro Package-Version. Mitigation: Kein weiterer Handlungsbedarf.
- **Server ohne Internet kann keine neuen Packages laden** → Packages, die einmal gecached sind, funktionieren weiterhin. Fehlermeldung ist klar.
- **Sicherheit: Packages können beliebigen Typst-Code enthalten** → Typst selbst ist sandboxed (kein Dateisystem-/Netzwerkzugriff aus Typst-Code). WASM-Plugins sind ein separates Thema und werden hier nicht unterstützt (Typst-Plugins erfordern zusätzliche `World`-Methoden).
- **Race Condition bei parallelen Downloads des gleichen Packages** → Mitigation: `Mutex` im `PackageCache` serialisiert Downloads. Im schlimmsten Fall wird doppelt heruntergeladen, aber der Cache ist konsistent.
