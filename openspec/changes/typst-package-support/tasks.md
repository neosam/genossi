## 1. Dependencies

- [x] 1.1 Add `flate2` und `tar` als Dependencies in `genossi_service_impl/Cargo.toml`
- [x] 1.2 Add `reqwest` mit `blocking`-Feature als Dependency in `genossi_service_impl/Cargo.toml`

## 2. PackageCache implementieren

- [x] 2.1 Erstelle `PackageCache`-Struct mit `cache_dir: PathBuf` und `downloaded: Mutex<HashSet<String>>` in `pdf_generation.rs`
- [x] 2.2 Implementiere `PackageCache::new()` mit Lesen von `TYPST_PACKAGE_CACHE` Environment-Variable (Default: `./typst-packages`)
- [x] 2.3 Implementiere `PackageCache::package_dir(&self, pkg: &PackageSpec) -> PathBuf` die den Cache-Pfad für ein Package zurückgibt
- [x] 2.4 Implementiere `PackageCache::ensure_downloaded(&self, pkg: &PackageSpec) -> Result<(), FileError>` die prüft ob das Package gecached ist und es bei Bedarf herunterlädt
- [x] 2.5 Implementiere die Download-Logik: HTTP GET auf `https://packages.typst.org/{namespace}/{name}-{version}.tar.gz`, Entpacken mit `flate2` + `tar` in den Cache-Pfad

## 3. TemplateWorld erweitern

- [x] 3.1 Füge `package_cache: &'a PackageCache` als Feld zu `TemplateWorld` hinzu
- [x] 3.2 Erweitere `resolve_path()` um Package-Erkennung: Wenn `id.package()` `Some` ist, löse über `package_cache.package_dir()` auf
- [x] 3.3 Erweitere `source()` um `package_cache.ensure_downloaded()` Aufruf vor dem Dateizugriff bei Package-FileIds
- [x] 3.4 Erweitere `file()` um `package_cache.ensure_downloaded()` Aufruf vor dem Dateizugriff bei Package-FileIds

## 4. PdfGenerator anpassen

- [x] 4.1 Füge `package_cache: PackageCache` als Feld zu `PdfGenerator` hinzu
- [x] 4.2 Initialisiere `PackageCache` in `PdfGenerator::new()`
- [x] 4.3 Übergib `&self.package_cache` an `TemplateWorld::new()`

## 5. Tests

- [x] 5.1 Unit-Test: `PackageCache::package_dir()` gibt korrekten Pfad zurück
- [x] 5.2 Unit-Test: `resolve_path()` mit Package-FileId löst in den Cache-Pfad auf
- [x] 5.3 Integrationstest: Template mit lokaler `_layout.typ`-Import funktioniert weiterhin (Regression)
- [x] 5.4 Integrationstest: Template mit Package-Import rendert erfolgreich (mit echtem Download von einem kleinen Package)
- [x] 5.5 Unit-Test: Fehlerbehandlung wenn Package nicht existiert (HTTP 404)
