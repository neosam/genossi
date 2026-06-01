use genossi_dao::assembly::AssemblyEntity;
use genossi_dao::attendance::AttendanceMemberRow;
use genossi_dao::member::MemberEntity;
use genossi_dao::repayment_phase::RepaymentPhaseEntity;
use genossi_service::application::Application;
use genossi_service::member::Member;
use genossi_service::repayment_context::RepaymentContext;
use genossi_service::template::TemplateError;
use genossi_service::ServiceError;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use typst::foundations::{Bytes, Dict, Str, Value};
use typst::layout::PagedDocument;
use typst::syntax::package::PackageSpec;
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

const EMBEDDED_FONTS: &[&[u8]] = &[
    include_bytes!("../../fonts/LiberationSans-Regular.ttf"),
    include_bytes!("../../fonts/LiberationSans-Bold.ttf"),
    include_bytes!("../../fonts/LiberationSans-Italic.ttf"),
    include_bytes!("../../fonts/LiberationSans-BoldItalic.ttf"),
];

/// Phase 11 (EXPO-02): Rendering-Bundle pro Auszahlungseintrag.
///
/// Pre-computed im Service-Impl (Plan 11.03); der Renderer baut nur die JSON-
/// Inputs fuer Typst. `amount_str` und `purpose` sind nach D-04 (Verwendungs-
/// zweck-Schema mit Umlauten) + Phase-10-D-04 (deutsche Euro-Formatierung
/// "12,00") konform vorbereitet.
///
/// D-05: Sonderzeichen ä/ö/ü/ß bleiben unveraendert — KEINE ASCII-
/// Sanitization. Typst rendert UTF-8 nativ.
/// D-07: `iban` ist leerer String, wenn `Member.bank_account == None`
/// (Konversion `unwrap_or_default()` im Service-Impl).
#[derive(Debug, Clone)]
pub struct RepaymentExportRow {
    pub member_number: i64,
    pub name: String,
    pub iban: String,
    pub share_count: i32,
    pub amount_str: String,
    pub purpose: String,
}

pub struct PackageCache {
    cache_dir: PathBuf,
    downloaded: Mutex<HashSet<String>>,
}

impl Default for PackageCache {
    fn default() -> Self {
        Self::new()
    }
}

impl PackageCache {
    pub fn new() -> Self {
        let cache_dir = std::env::var("TYPST_PACKAGE_CACHE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./typst-packages"));
        Self {
            cache_dir,
            downloaded: Mutex::new(HashSet::new()),
        }
    }

    fn package_dir(&self, pkg: &PackageSpec) -> PathBuf {
        self.cache_dir
            .join(pkg.namespace.as_str())
            .join(pkg.name.as_str())
            .join(pkg.version.to_string())
    }

    fn package_key(pkg: &PackageSpec) -> String {
        format!("@{}/{}:{}", pkg.namespace, pkg.name, pkg.version)
    }

    fn ensure_downloaded(&self, pkg: &PackageSpec) -> Result<(), typst::diag::FileError> {
        let key = Self::package_key(pkg);
        let dir = self.package_dir(pkg);

        // Fast path: already known to be downloaded
        if self.downloaded.lock().unwrap().contains(&key) {
            return Ok(());
        }

        // Check filesystem
        if dir.exists() {
            self.downloaded.lock().unwrap().insert(key);
            return Ok(());
        }

        // Download in a dedicated thread to avoid blocking the async runtime.
        // reqwest::blocking internally creates its own runtime, which panics
        // if called from within an existing Tokio runtime.
        let pkg_clone = pkg.clone();
        let dir_clone = dir.clone();
        std::thread::spawn(move || Self::download_package_impl(&pkg_clone, &dir_clone))
            .join()
            .map_err(|_| {
                typst::diag::FileError::Other(Some(typst::diag::EcoString::from(
                    "Package download thread panicked",
                )))
            })?
            .map_err(|e| typst::diag::FileError::Other(Some(typst::diag::EcoString::from(e))))?;

        self.downloaded.lock().unwrap().insert(key);
        Ok(())
    }

    fn download_package_impl(pkg: &PackageSpec, target_dir: &Path) -> Result<(), String> {
        let url = format!(
            "https://packages.typst.org/{}/{}-{}.tar.gz",
            pkg.namespace, pkg.name, pkg.version
        );

        let response = reqwest::blocking::get(&url).map_err(|e| {
            format!(
                "Failed to download package {}: {}",
                Self::package_key(pkg),
                e
            )
        })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(format!("Package {} not found", Self::package_key(pkg)));
        }

        if !response.status().is_success() {
            return Err(format!(
                "Failed to download package {}: HTTP {}",
                Self::package_key(pkg),
                response.status()
            ));
        }

        let bytes = response
            .bytes()
            .map_err(|e| format!("Failed to read package data: {}", e))?;

        std::fs::create_dir_all(target_dir)
            .map_err(|e| format!("Failed to create cache directory: {}", e))?;

        let decoder = flate2::read::GzDecoder::new(&bytes[..]);
        let mut archive = tar::Archive::new(decoder);
        archive
            .unpack(target_dir)
            .map_err(|e| format!("Failed to extract package: {}", e))?;

        Ok(())
    }
}

pub struct PdfGenerator {
    fonts: Vec<Font>,
    book: LazyHash<FontBook>,
    package_cache: PackageCache,
}

impl Default for PdfGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfGenerator {
    pub fn new() -> Self {
        let mut fonts = Vec::new();

        // Load embedded fonts
        for data in EMBEDDED_FONTS {
            let bytes = Bytes::new(data.to_vec());
            for font in Font::iter(bytes) {
                fonts.push(font);
            }
        }

        let book = LazyHash::new(FontBook::from_fonts(fonts.iter()));

        Self {
            fonts,
            book,
            package_cache: PackageCache::new(),
        }
    }

    /// Render a Typst template to PDF bytes.
    pub fn render(
        &self,
        template_path: &str,
        template_base: &Path,
        member: &Member,
    ) -> Result<Vec<u8>, TemplateError> {
        // Read the main template file
        let full_path = template_base.join(template_path);
        let source_text = std::fs::read_to_string(&full_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                TemplateError::NotFound
            } else {
                TemplateError::IoError(Arc::from(e.to_string()))
            }
        })?;

        // Build member data as JSON string for sys.inputs
        let inputs = self.build_inputs(member);

        // Create the world
        let world = TemplateWorld::new(
            &source_text,
            template_path,
            template_base.to_path_buf(),
            inputs,
            &self.fonts,
            &self.book,
            &self.package_cache,
        );

        // Compile
        let result = typst::compile::<PagedDocument>(&world);

        match result.output {
            Ok(document) => {
                let options = typst_pdf::PdfOptions::default();
                let pdf_bytes = typst_pdf::pdf(&document, &options)
                    .map_err(|e| TemplateError::RenderError(Arc::from(format!("{:?}", e))))?;
                Ok(pdf_bytes)
            }
            Err(diagnostics) => {
                let errors: Vec<String> = diagnostics
                    .iter()
                    .map(|d| {
                        let msg = &d.message;
                        format!("{}", msg)
                    })
                    .collect();
                Err(TemplateError::RenderError(Arc::from(errors.join("\n"))))
            }
        }
    }

    /// Render a Typst template to PDF bytes using application data.
    pub fn render_application(
        &self,
        template_path: &str,
        template_base: &Path,
        application: &Application,
    ) -> Result<Vec<u8>, TemplateError> {
        let full_path = template_base.join(template_path);
        let source_text = std::fs::read_to_string(&full_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                TemplateError::NotFound
            } else {
                TemplateError::IoError(Arc::from(e.to_string()))
            }
        })?;

        let inputs = self.build_inputs_application(application);

        let world = TemplateWorld::new(
            &source_text,
            template_path,
            template_base.to_path_buf(),
            inputs,
            &self.fonts,
            &self.book,
            &self.package_cache,
        );

        let result = typst::compile::<PagedDocument>(&world);

        match result.output {
            Ok(document) => {
                let options = typst_pdf::PdfOptions::default();
                let pdf_bytes = typst_pdf::pdf(&document, &options)
                    .map_err(|e| TemplateError::RenderError(Arc::from(format!("{:?}", e))))?;
                Ok(pdf_bytes)
            }
            Err(diagnostics) => {
                let errors: Vec<String> = diagnostics
                    .iter()
                    .map(|d| {
                        let msg = &d.message;
                        format!("{}", msg)
                    })
                    .collect();
                Err(TemplateError::RenderError(Arc::from(errors.join("\n"))))
            }
        }
    }

    /// Render the Teilnehmerliste template (Phase 6, D-04 / D-08 / D-10) to
    /// PDF bytes.
    ///
    /// Inputs:
    ///   * `template_path` — filename relative to `template_base`, e.g.
    ///     `"teilnehmerliste.typ"`.
    ///   * `template_base` — root directory containing the Typst template
    ///     (resolves `#import "_layout.typ"`).
    ///   * `assembly` — provides `assembly.name` (-> JSON key `title`) and
    ///     `assembly.date` (-> JSON key `date`, formatted DD.MM.YYYY).
    ///   * `rows` — already filtered + sorted rows from
    ///     `AttendanceDao::list_members_for_assembly` (DSGVO 7-col whitelist).
    ///   * `present` — pre-computed count of `is_present == true` rows.
    ///   * `total` — optional total. `Some(n)` when `include == All`
    ///     (`n == rows.len()`); `None` when `include == Present` (the
    ///     "X anwesend" variant in the template).
    ///
    /// Wraps `TemplateError` into `ServiceError::InternalError` so the
    /// AttendanceExportServiceImpl can `?`-propagate the result. The
    /// underlying Typst pipeline is the same as `render_application`.
    pub fn render_attendance_list(
        &self,
        template_path: &str,
        template_base: &Path,
        assembly: &AssemblyEntity,
        rows: &[AttendanceMemberRow],
        present: u64,
        total: Option<u64>,
    ) -> Result<Vec<u8>, ServiceError> {
        let full_path = template_base.join(template_path);
        let source_text = std::fs::read_to_string(&full_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ServiceError::InternalError(Arc::from(format!(
                    "template not found: {}",
                    full_path.display()
                )))
            } else {
                ServiceError::InternalError(Arc::from(format!("template io error: {}", e)))
            }
        })?;

        let inputs = build_inputs_attendance(assembly, rows, present, total);

        let world = TemplateWorld::new(
            &source_text,
            template_path,
            template_base.to_path_buf(),
            inputs,
            &self.fonts,
            &self.book,
            &self.package_cache,
        );

        let result = typst::compile::<PagedDocument>(&world);

        match result.output {
            Ok(document) => {
                let options = typst_pdf::PdfOptions::default();
                let pdf_bytes = typst_pdf::pdf(&document, &options).map_err(|e| {
                    ServiceError::InternalError(Arc::from(format!(
                        "typst pdf serialisation failed: {:?}",
                        e
                    )))
                })?;
                Ok(pdf_bytes)
            }
            Err(diagnostics) => {
                let errors: Vec<String> = diagnostics
                    .iter()
                    .map(|d| format!("{}", d.message))
                    .collect();
                Err(ServiceError::InternalError(Arc::from(format!(
                    "typst compile errors: {}",
                    errors.join("\n")
                ))))
            }
        }
    }

    /// Phase 11 (EXPO-02): Renders die Auszahlungsliste fuer eine
    /// RepaymentPhase als PDF.
    ///
    /// Inputs:
    ///   * `template_path` — filename relative zu `template_base`, z. B.
    ///     `"auszahlungsliste.typ"`.
    ///   * `template_base` — root directory (resolves `#import "_layout.typ"`).
    ///   * `phase` — liefert `phase.fiscal_year` und `phase.share_value` fuer
    ///     `meta.title`, `meta.fiscal_year` und Summe `meta.total_amount_str`.
    ///   * `rows` — pre-computed RepaymentExportRows (D-04 purpose-String inkl.
    ///     Umlaute, D-05 no-sanitization).
    ///
    /// Wraps Typst-Fehler in `ServiceError::InternalError`; gleiches Pipeline-
    /// Pattern wie `render_attendance_list`.
    pub fn render_repayment_list(
        &self,
        template_path: &str,
        template_base: &Path,
        phase: &RepaymentPhaseEntity,
        rows: &[RepaymentExportRow],
    ) -> Result<Vec<u8>, ServiceError> {
        let full_path = template_base.join(template_path);
        let source_text = std::fs::read_to_string(&full_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ServiceError::InternalError(Arc::from(format!(
                    "template not found: {}",
                    full_path.display()
                )))
            } else {
                ServiceError::InternalError(Arc::from(format!("template io error: {}", e)))
            }
        })?;

        let inputs = build_inputs_repayment(phase, rows);

        let world = TemplateWorld::new(
            &source_text,
            template_path,
            template_base.to_path_buf(),
            inputs,
            &self.fonts,
            &self.book,
            &self.package_cache,
        );

        let result = typst::compile::<PagedDocument>(&world);
        let document = match result.output {
            Ok(doc) => doc,
            Err(diagnostics) => {
                let messages: Vec<String> = diagnostics
                    .iter()
                    .map(|d| format!("{}", d.message))
                    .collect();
                return Err(ServiceError::InternalError(Arc::from(format!(
                    "typst compile errors: {}",
                    messages.join("\n")
                ))));
            }
        };

        let pdf_bytes =
            typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).map_err(|e| {
                ServiceError::InternalError(Arc::from(format!(
                    "typst pdf serialisation failed: {:?}",
                    e
                )))
            })?;

        Ok(pdf_bytes)
    }

    fn build_inputs_application(&self, application: &Application) -> Dict {
        let mut inputs = Dict::new();

        let mut app_map = serde_json::Map::new();
        app_map.insert(
            "first_name".to_string(),
            serde_json::Value::String(application.first_name.to_string()),
        );
        app_map.insert(
            "last_name".to_string(),
            serde_json::Value::String(application.last_name.to_string()),
        );
        app_map.insert(
            "salutation".to_string(),
            application
                .salutation
                .as_ref()
                .map(|v| serde_json::Value::String(v.as_str().to_string()))
                .unwrap_or(serde_json::Value::Null),
        );
        app_map.insert(
            "title".to_string(),
            application
                .title
                .as_ref()
                .map(|v| serde_json::Value::String(v.to_string()))
                .unwrap_or(serde_json::Value::Null),
        );
        app_map.insert(
            "email".to_string(),
            application
                .email
                .as_ref()
                .map(|v| serde_json::Value::String(v.to_string()))
                .unwrap_or(serde_json::Value::Null),
        );
        app_map.insert(
            "street".to_string(),
            application
                .street
                .as_ref()
                .map(|v| serde_json::Value::String(v.to_string()))
                .unwrap_or(serde_json::Value::Null),
        );
        app_map.insert(
            "house_number".to_string(),
            application
                .house_number
                .as_ref()
                .map(|v| serde_json::Value::String(v.to_string()))
                .unwrap_or(serde_json::Value::Null),
        );
        app_map.insert(
            "postal_code".to_string(),
            application
                .postal_code
                .as_ref()
                .map(|v| serde_json::Value::String(v.to_string()))
                .unwrap_or(serde_json::Value::Null),
        );
        app_map.insert(
            "city".to_string(),
            application
                .city
                .as_ref()
                .map(|v| serde_json::Value::String(v.to_string()))
                .unwrap_or(serde_json::Value::Null),
        );
        app_map.insert(
            "shares".to_string(),
            serde_json::Value::Number(serde_json::Number::from(application.shares)),
        );
        app_map.insert(
            "status".to_string(),
            serde_json::Value::String(application.status.as_str().to_string()),
        );

        let format = time::format_description::parse("[day].[month].[year]").expect("valid format");
        app_map.insert(
            "created".to_string(),
            serde_json::Value::String(
                application
                    .created
                    .date()
                    .format(&format)
                    .unwrap_or_default(),
            ),
        );

        let app_json = serde_json::to_string(&serde_json::Value::Object(app_map)).unwrap();
        inputs.insert(
            Str::from("application"),
            Value::Str(Str::from(app_json.as_str())),
        );

        let today = time::OffsetDateTime::now_utc().date();
        let today_str = today.format(&format).unwrap_or_default();
        inputs.insert(
            Str::from("today"),
            Value::Str(Str::from(today_str.as_str())),
        );

        inputs
    }

    fn build_inputs(&self, member: &Member) -> Dict {
        let mut inputs = Dict::new();

        // Build member data as JSON string
        let mut member_map = serde_json::Map::new();
        member_map.insert(
            "first_name".to_string(),
            serde_json::Value::String(member.first_name.to_string()),
        );
        member_map.insert(
            "last_name".to_string(),
            serde_json::Value::String(member.last_name.to_string()),
        );
        member_map.insert(
            "salutation".to_string(),
            member
                .salutation
                .as_ref()
                .map(|v| serde_json::Value::String(v.as_str().to_string()))
                .unwrap_or(serde_json::Value::Null),
        );
        member_map.insert(
            "title".to_string(),
            member
                .title
                .as_ref()
                .map(|v| serde_json::Value::String(v.to_string()))
                .unwrap_or(serde_json::Value::Null),
        );
        member_map.insert(
            "member_number".to_string(),
            serde_json::Value::Number(serde_json::Number::from(member.member_number)),
        );
        member_map.insert(
            "email".to_string(),
            member
                .email
                .as_ref()
                .map(|v| serde_json::Value::String(v.to_string()))
                .unwrap_or(serde_json::Value::Null),
        );
        member_map.insert(
            "company".to_string(),
            member
                .company
                .as_ref()
                .map(|v| serde_json::Value::String(v.to_string()))
                .unwrap_or(serde_json::Value::Null),
        );
        member_map.insert(
            "comment".to_string(),
            member
                .comment
                .as_ref()
                .map(|v| serde_json::Value::String(v.to_string()))
                .unwrap_or(serde_json::Value::Null),
        );
        member_map.insert(
            "street".to_string(),
            member
                .street
                .as_ref()
                .map(|v| serde_json::Value::String(v.to_string()))
                .unwrap_or(serde_json::Value::Null),
        );
        member_map.insert(
            "house_number".to_string(),
            member
                .house_number
                .as_ref()
                .map(|v| serde_json::Value::String(v.to_string()))
                .unwrap_or(serde_json::Value::Null),
        );
        member_map.insert(
            "postal_code".to_string(),
            member
                .postal_code
                .as_ref()
                .map(|v| serde_json::Value::String(v.to_string()))
                .unwrap_or(serde_json::Value::Null),
        );
        member_map.insert(
            "city".to_string(),
            member
                .city
                .as_ref()
                .map(|v| serde_json::Value::String(v.to_string()))
                .unwrap_or(serde_json::Value::Null),
        );
        member_map.insert(
            "bank_account".to_string(),
            member
                .bank_account
                .as_ref()
                .map(|v| serde_json::Value::String(v.to_string()))
                .unwrap_or(serde_json::Value::Null),
        );

        // Format dates
        let format = time::format_description::parse("[day].[month].[year]").expect("valid format");
        member_map.insert(
            "join_date".to_string(),
            serde_json::Value::String(member.join_date.format(&format).unwrap_or_default()),
        );
        member_map.insert(
            "exit_date".to_string(),
            member
                .exit_date
                .map(|d| serde_json::Value::String(d.format(&format).unwrap_or_default()))
                .unwrap_or(serde_json::Value::Null),
        );

        member_map.insert(
            "shares_at_joining".to_string(),
            serde_json::Value::Number(serde_json::Number::from(member.shares_at_joining)),
        );
        member_map.insert(
            "current_shares".to_string(),
            serde_json::Value::Number(serde_json::Number::from(member.current_shares)),
        );
        member_map.insert(
            "current_balance".to_string(),
            serde_json::Value::Number(serde_json::Number::from(member.current_balance)),
        );
        member_map.insert(
            "migrated".to_string(),
            serde_json::Value::Bool(member.migrated),
        );

        let member_json = serde_json::to_string(&serde_json::Value::Object(member_map)).unwrap();
        inputs.insert(
            Str::from("member"),
            Value::Str(Str::from(member_json.as_str())),
        );

        // Add today's date
        let today = time::OffsetDateTime::now_utc().date();
        let today_str = today.format(&format).unwrap_or_default();
        inputs.insert(
            Str::from("today"),
            Value::Str(Str::from(today_str.as_str())),
        );

        inputs
    }
}

/// Build the Typst `sys.inputs` dict for the Teilnehmerliste template.
///
/// Produces two string-keyed entries:
///   * `meta` — JSON of `{ title, date, present, total }`. `total` is
///     serialized as null when `None` (D-10: only present-count for
///     `include=Present`). The JSON key remains `title` because the Typst
///     template binds to `meta.title` (the source field on the Rust side is
///     `assembly.name`).
///   * `rows` — JSON array of `{ member_number, first_name, last_name,
///     salutation, title, is_present }`. `member_id` is intentionally
///     excluded (DSGVO PII minimization — auditors get the visible 6
///     columns, the internal ID never reaches PDF).
fn build_inputs_attendance(
    assembly: &AssemblyEntity,
    rows: &[AttendanceMemberRow],
    present: u64,
    total: Option<u64>,
) -> Dict {
    let mut inputs = Dict::new();

    // Format the assembly date as DD.MM.YYYY (D-15-adjacent: same Punkt-
    // Pattern as `render_application` for visual consistency in the PDF).
    let german_date_fmt =
        time::format_description::parse("[day].[month].[year]").expect("static format");
    let date_str = assembly
        .date
        .date()
        .format(&german_date_fmt)
        .unwrap_or_else(|_| "unbekannt".to_string());

    let total_value = match total {
        Some(n) => serde_json::Value::Number(serde_json::Number::from(n)),
        None => serde_json::Value::Null,
    };

    let meta = serde_json::json!({
        "title": assembly.name.as_ref(),
        "date": date_str,
        "present": present,
        "total": total_value,
    });
    let meta_json = serde_json::to_string(&meta).expect("meta json serialisable");
    inputs.insert(Str::from("meta"), Value::Str(Str::from(meta_json.as_str())));

    let row_values: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "member_number": r.member_number,
                "first_name": r.first_name.as_ref(),
                "last_name": r.last_name.as_ref(),
                "salutation": r.salutation.as_ref().map(|s| s.as_ref()),
                "title": r.title.as_ref().map(|s| s.as_ref()),
                "is_present": r.is_present,
            })
        })
        .collect();
    let rows_json = serde_json::to_string(&serde_json::Value::Array(row_values))
        .expect("rows json serialisable");
    inputs.insert(Str::from("rows"), Value::Str(Str::from(rows_json.as_str())));

    inputs
}

/// Phase 11 (EXPO-02): Build the Typst `sys.inputs` dict for
/// `auszahlungsliste.typ`.
///
/// Produces two string-keyed entries:
///   * `meta` — JSON of `{ title, date, fiscal_year, row_count,
///     total_amount_str, phase_id }`. `total_amount_str` is the sum of
///     `share_count * phase.share_value` formatted as deutsche Euro-String
///     (z. B. `"360,00"`).
///   * `rows` — JSON array of `{ member_number, name, iban, share_count,
///     amount_str, purpose }`. `purpose` and `amount_str` come pre-computed
///     from the service (D-04 / Phase-10-D-04).
///
/// REVISION-Fix B3 (Phase-10-D-04-Pattern-Konsistenz): Summe via
/// `format!("{},{:02}", cents / 100, cents % 100)` OHNE `.abs()` — Domain-
/// Constraint `share_count_to_pay_out >= 0` und `share_value > 0` garantieren
/// non-negative `total_cents`. `.abs()` wuerde Inkonsistenz mit PATTERNS.md §S9
/// und Phase-10-D-04 einfuehren.
fn build_inputs_repayment(
    phase: &RepaymentPhaseEntity,
    rows: &[RepaymentExportRow],
) -> Dict {
    let mut inputs = Dict::new();

    // ISO date string (heute, UTC). Plan 11.03 nutzt eine eigene Quelle (z. B.
    // `phase.opened_at`); fuer die reine Render-Foundation reicht today.
    let date_str = time::OffsetDateTime::now_utc().date().to_string();

    // Total amount in cents: SUM(share_count × share_value_cent), formatted
    // als deutscher Euro-String "EUR,CC".
    // Cast über i64, um Multiplikations-Overflow bei realistischen Phasen-
    // groessen (50-100 Eintraege × 4-stelliger share_count × i64-cent) zu
    // vermeiden.
    let total_cents: i64 = rows
        .iter()
        .map(|r| (r.share_count as i64) * phase.share_value)
        .sum();
    let total_amount_str = format!("{},{:02}", total_cents / 100, total_cents % 100);

    let meta = serde_json::json!({
        "title": format!("Auszahlungsliste Geschaeftsjahr {}", phase.fiscal_year),
        "date": date_str,
        "fiscal_year": phase.fiscal_year,
        "row_count": rows.len(),
        "total_amount_str": total_amount_str,
        "phase_id": phase.id.to_string(),
    });
    let meta_json = serde_json::to_string(&meta).expect("meta json serialisable");
    inputs.insert(Str::from("meta"), Value::Str(Str::from(meta_json.as_str())));

    let row_values: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "member_number": r.member_number,
                "name": r.name,
                "iban": r.iban,
                "share_count": r.share_count,
                "amount_str": r.amount_str,
                "purpose": r.purpose,
            })
        })
        .collect();
    let rows_json = serde_json::to_string(&serde_json::Value::Array(row_values))
        .expect("rows json serialisable");
    inputs.insert(Str::from("rows"), Value::Str(Str::from(rows_json.as_str())));

    inputs
}

/// Phase 13 D-13-01: Inputs fuer EIN Single-Letter-PDF (1 Member).
///
/// JSON-Shape spiegelt das Default-Template
/// `templates/defaults/auszahlungs_anschreiben.typ`. Drei Top-Level-Keys
/// `member`, `repayment`, `today` werden als JSON-String unter
/// `sys.inputs.<key>` an Typst gereicht.
///
/// Pitfall #5 (RESEARCH): `member.bank_account = None` muss als JSON `null`
/// serialisiert werden — das Template hat einen `#if m.bank_account != none`
/// -Switch. KEIN Default-String fuer abwesende IBAN.
///
/// `_phase` ist heute ungenutzt (Subject/Sender/Recipient kommen aus Member +
/// RepaymentContext); wird mitgereicht fuer kuenftige Erweiterungen
/// (z.B. phase-spezifische Anrede).
fn build_inputs_repayment_letter(
    _phase: &RepaymentPhaseEntity,
    member: &MemberEntity,
    ctx: &RepaymentContext,
) -> Dict {
    let mut inputs = Dict::new();

    let date_str = time::OffsetDateTime::now_utc().date().to_string();
    inputs.insert(
        Str::from("today"),
        Value::Str(Str::from(date_str.as_str())),
    );

    let member_json = serde_json::json!({
        "member_number": member.member_number,
        "salutation": member.salutation.as_ref().map(|s| s.as_str()),
        "title": member.title.as_ref().map(|s| s.as_ref()),
        "first_name": member.first_name.as_ref(),
        "last_name": member.last_name.as_ref(),
        "street": member.street.as_ref().map(|s| s.as_ref()),
        "house_number": member.house_number.as_ref().map(|s| s.as_ref()),
        "postal_code": member.postal_code.as_ref().map(|s| s.as_ref()),
        "city": member.city.as_ref().map(|s| s.as_ref()),
        "bank_account": member.bank_account.as_ref().map(|s| s.as_ref()),
    });
    let member_str = serde_json::to_string(&member_json).expect("member json serialisable");
    inputs.insert(
        Str::from("member"),
        Value::Str(Str::from(member_str.as_str())),
    );

    let repayment_json = serde_json::json!({
        "share_count": ctx.share_count,
        "payout_amount": ctx.payout_amount,
        "fiscal_year": ctx.fiscal_year,
    });
    let repayment_str =
        serde_json::to_string(&repayment_json).expect("repayment json serialisable");
    inputs.insert(
        Str::from("repayment"),
        Value::Str(Str::from(repayment_str.as_str())),
    );

    inputs
}

/// Phase 13 D-13-01: Inputs fuer Bundle-PDF (N Members in EINEM Compile).
///
/// `recipients`-Array enthaelt pro Member ein Objekt mit `member` und
/// `repayment` Sub-JSON. Reihenfolge bestimmt der Caller (Service sortiert nach
/// `member_number` ASC — RESEARCH Pitfall #10).
///
/// Zusaetzlich `today` und `meta` (mit `phase_id`, `fiscal_year`,
/// `recipient_count`) — `meta` ist heute fuer das Bundle-Template optional, wird
/// aber fuer kuenftige Header/Trailer-Erweiterungen bereitgestellt.
fn build_inputs_repayment_letters_bundle(
    phase: &RepaymentPhaseEntity,
    recipients: &[(MemberEntity, RepaymentContext)],
) -> Dict {
    let mut inputs = Dict::new();

    let date_str = time::OffsetDateTime::now_utc().date().to_string();
    inputs.insert(
        Str::from("today"),
        Value::Str(Str::from(date_str.as_str())),
    );

    let meta = serde_json::json!({
        "fiscal_year": phase.fiscal_year,
        "recipient_count": recipients.len(),
        "phase_id": phase.id.to_string(),
    });
    inputs.insert(
        Str::from("meta"),
        Value::Str(Str::from(
            serde_json::to_string(&meta)
                .expect("meta json serialisable")
                .as_str(),
        )),
    );

    let recipient_values: Vec<serde_json::Value> = recipients
        .iter()
        .map(|(member, ctx)| {
            serde_json::json!({
                "member": {
                    "member_number": member.member_number,
                    "salutation": member.salutation.as_ref().map(|s| s.as_str()),
                    "title": member.title.as_ref().map(|s| s.as_ref()),
                    "first_name": member.first_name.as_ref(),
                    "last_name": member.last_name.as_ref(),
                    "street": member.street.as_ref().map(|s| s.as_ref()),
                    "house_number": member.house_number.as_ref().map(|s| s.as_ref()),
                    "postal_code": member.postal_code.as_ref().map(|s| s.as_ref()),
                    "city": member.city.as_ref().map(|s| s.as_ref()),
                    "bank_account": member.bank_account.as_ref().map(|s| s.as_ref()),
                },
                "repayment": {
                    "share_count": ctx.share_count,
                    "payout_amount": ctx.payout_amount,
                    "fiscal_year": ctx.fiscal_year,
                },
            })
        })
        .collect();
    let recipients_str =
        serde_json::to_string(&serde_json::Value::Array(recipient_values))
            .expect("recipients json serialisable");
    inputs.insert(
        Str::from("recipients"),
        Value::Str(Str::from(recipients_str.as_str())),
    );

    inputs
}

struct TemplateWorld<'a> {
    library: LazyHash<Library>,
    book: &'a LazyHash<FontBook>,
    fonts: &'a [Font],
    main_source: Source,
    template_base: PathBuf,
    package_cache: &'a PackageCache,
    source_cache: std::sync::Mutex<HashMap<FileId, Source>>,
}

impl<'a> TemplateWorld<'a> {
    fn new(
        source_text: &str,
        template_path: &str,
        template_base: PathBuf,
        inputs: Dict,
        fonts: &'a [Font],
        book: &'a LazyHash<FontBook>,
        package_cache: &'a PackageCache,
    ) -> Self {
        let main_id = FileId::new(None, VirtualPath::new(template_path));
        let main_source = Source::new(main_id, source_text.to_string());

        let library = Library::builder().with_inputs(inputs).build();

        Self {
            library: LazyHash::new(library),
            book,
            fonts,
            main_source,
            template_base,
            package_cache,
            source_cache: std::sync::Mutex::new(HashMap::new()),
        }
    }

    fn resolve_path(&self, id: FileId) -> PathBuf {
        let vpath = id.vpath();
        let relative = vpath.as_rootless_path();
        if let Some(pkg) = id.package() {
            self.package_cache.package_dir(pkg).join(relative)
        } else {
            self.template_base.join(relative)
        }
    }
}

impl World for TemplateWorld<'_> {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.book
    }

    fn main(&self) -> FileId {
        self.main_source.id()
    }

    fn source(&self, id: FileId) -> typst::diag::FileResult<Source> {
        if id == self.main_source.id() {
            return Ok(self.main_source.clone());
        }

        // Check cache
        if let Some(source) = self.source_cache.lock().unwrap().get(&id) {
            return Ok(source.clone());
        }

        // Ensure package is downloaded if this is a package file
        if let Some(pkg) = id.package() {
            self.package_cache.ensure_downloaded(pkg)?;
        }

        // Read from filesystem
        let path = self.resolve_path(id);
        let text = std::fs::read_to_string(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                typst::diag::FileError::NotFound(path)
            } else {
                typst::diag::FileError::Other(Some(typst::diag::EcoString::from(e.to_string())))
            }
        })?;

        let source = Source::new(id, text);
        self.source_cache.lock().unwrap().insert(id, source.clone());
        Ok(source)
    }

    fn file(&self, id: FileId) -> typst::diag::FileResult<Bytes> {
        // Ensure package is downloaded if this is a package file
        if let Some(pkg) = id.package() {
            self.package_cache.ensure_downloaded(pkg)?;
        }

        let path = self.resolve_path(id);
        let data = std::fs::read(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                typst::diag::FileError::NotFound(path)
            } else {
                typst::diag::FileError::Other(Some(typst::diag::EcoString::from(e.to_string())))
            }
        })?;
        Ok(Bytes::new(data))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<typst::foundations::Datetime> {
        let now = time::OffsetDateTime::now_utc();
        typst::foundations::Datetime::from_ymd(now.year(), now.month() as u8, now.day())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::TempDir;
    use typst::syntax::package::PackageVersion;
    use uuid::Uuid;

    fn test_package_spec() -> PackageSpec {
        PackageSpec {
            namespace: typst::diag::EcoString::from("preview"),
            name: typst::diag::EcoString::from("example"),
            version: PackageVersion {
                major: 1,
                minor: 0,
                patch: 0,
            },
        }
    }

    fn test_member() -> Member {
        Member {
            id: Uuid::new_v4(),
            member_number: 1001,
            first_name: Arc::from("Max"),
            last_name: Arc::from("Mustermann"),
            salutation: None,
            title: None,
            email: Some(Arc::from("max@example.com")),
            company: None,
            comment: None,
            street: Some(Arc::from("Musterstraße")),
            house_number: Some(Arc::from("42")),
            postal_code: Some(Arc::from("12345")),
            city: Some(Arc::from("Musterstadt")),
            join_date: time::Date::from_calendar_date(2024, time::Month::January, 15).unwrap(),
            shares_at_joining: 3,
            current_shares: 5,
            current_balance: 15000,
            action_count: 2,
            migrated: false,
            exit_date: None,
            bank_account: None,
            status: genossi_dao::member::MemberStatus::Normal,
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2024, time::Month::January, 15).unwrap(),
                time::Time::from_hms(10, 0, 0).unwrap(),
            ),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn test_application() -> Application {
        Application {
            id: Uuid::new_v4(),
            first_name: Arc::from("Erika"),
            last_name: Arc::from("Musterfrau"),
            salutation: Some(genossi_dao::member::Salutation::Frau),
            title: Some(Arc::from("Dr.")),
            email: Some(Arc::from("erika@example.com")),
            street: Some(Arc::from("Testweg")),
            house_number: Some(Arc::from("7")),
            postal_code: Some(Arc::from("54321")),
            city: Some(Arc::from("Teststadt")),
            shares: 3,
            status: genossi_dao::application::ApplicationStatus::Offen,
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2026, time::Month::March, 10).unwrap(),
                time::Time::from_hms(14, 30, 0).unwrap(),
            ),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    #[test]
    fn test_build_inputs_application_fields() {
        let generator = PdfGenerator::new();
        let app = test_application();
        let inputs = generator.build_inputs_application(&app);

        let app_str = match inputs.get(&Str::from("application")).unwrap() {
            Value::Str(s) => s.to_string(),
            _ => panic!("Expected string"),
        };
        let parsed: serde_json::Value = serde_json::from_str(&app_str).unwrap();

        assert_eq!(parsed["first_name"], "Erika");
        assert_eq!(parsed["last_name"], "Musterfrau");
        assert_eq!(parsed["salutation"], "Frau");
        assert_eq!(parsed["title"], "Dr.");
        assert_eq!(parsed["email"], "erika@example.com");
        assert_eq!(parsed["street"], "Testweg");
        assert_eq!(parsed["house_number"], "7");
        assert_eq!(parsed["postal_code"], "54321");
        assert_eq!(parsed["city"], "Teststadt");
        assert_eq!(parsed["shares"], 3);
        assert_eq!(parsed["status"], "Offen");
        assert_eq!(parsed["created"], "10.03.2026");

        // today should also be present
        assert!(inputs.get(&Str::from("today")).is_ok());
    }

    #[test]
    fn test_build_inputs_application_optional_fields_null() {
        let generator = PdfGenerator::new();
        let app = Application {
            id: Uuid::new_v4(),
            first_name: Arc::from("Max"),
            last_name: Arc::from("Test"),
            salutation: None,
            title: None,
            email: None,
            street: None,
            house_number: None,
            postal_code: None,
            city: None,
            shares: 1,
            status: genossi_dao::application::ApplicationStatus::Offen,
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
                time::Time::from_hms(0, 0, 0).unwrap(),
            ),
            deleted: None,
            version: Uuid::new_v4(),
        };
        let inputs = generator.build_inputs_application(&app);

        let app_str = match inputs.get(&Str::from("application")).unwrap() {
            Value::Str(s) => s.to_string(),
            _ => panic!("Expected string"),
        };
        let parsed: serde_json::Value = serde_json::from_str(&app_str).unwrap();

        assert!(parsed["salutation"].is_null());
        assert!(parsed["title"].is_null());
        assert!(parsed["email"].is_null());
        assert!(parsed["street"].is_null());
        assert!(parsed["house_number"].is_null());
        assert!(parsed["postal_code"].is_null());
        assert!(parsed["city"].is_null());
    }

    #[test]
    fn test_render_application_simple_template() {
        let dir = TempDir::new().unwrap();
        let template_content = r#"
#set page(paper: "a4")
#set text(size: 12pt)
#let app = json.decode(sys.inputs.at("application"))
Name: #app.first_name #app.last_name
Anteile: #app.shares
Status: #app.status
"#;

        std::fs::write(dir.path().join("test.typ"), template_content).unwrap();

        let generator = PdfGenerator::new();
        let app = test_application();
        let result = generator.render_application("test.typ", dir.path(), &app);

        assert!(result.is_ok());
        let pdf = result.unwrap();
        assert!(pdf.starts_with(b"%PDF"));
    }

    #[test]
    fn test_render_application_nonexistent_template() {
        let dir = TempDir::new().unwrap();
        let generator = PdfGenerator::new();
        let app = test_application();
        let result = generator.render_application("nonexistent.typ", dir.path(), &app);

        assert!(matches!(result, Err(TemplateError::NotFound)));
    }

    #[test]
    fn test_render_simple_template() {
        let dir = TempDir::new().unwrap();
        let template_content = r#"
#set page(paper: "a4")
#set text(size: 12pt)
#let member = json.decode(sys.inputs.at("member"))
Hello #member.first_name #member.last_name!
Member number: #member.member_number
"#;

        std::fs::write(dir.path().join("test.typ"), template_content).unwrap();

        let generator = PdfGenerator::new();
        let member = test_member();
        let result = generator.render("test.typ", dir.path(), &member);

        assert!(result.is_ok());
        let pdf = result.unwrap();
        // PDF should start with %PDF
        assert!(pdf.starts_with(b"%PDF"));
    }

    #[test]
    fn test_render_with_import() {
        let dir = TempDir::new().unwrap();

        let layout_content = r#"
#let greet(name) = {
  text(weight: "bold")[Hello, #name!]
}
"#;
        std::fs::write(dir.path().join("_layout.typ"), layout_content).unwrap();

        let main_content = r#"
#import "_layout.typ": greet
#let member = json.decode(sys.inputs.at("member"))
#greet(member.first_name)
"#;
        std::fs::write(dir.path().join("main.typ"), main_content).unwrap();

        let generator = PdfGenerator::new();
        let member = test_member();
        let result = generator.render("main.typ", dir.path(), &member);

        assert!(result.is_ok());
    }

    #[test]
    fn test_render_with_subdirectory_import() {
        let dir = TempDir::new().unwrap();

        let layout_content = r#"
#let title(text_content) = {
  text(size: 14pt, weight: "bold")[#text_content]
}
"#;
        std::fs::write(dir.path().join("_layout.typ"), layout_content).unwrap();

        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let sub_content = r#"
#import "../_layout.typ": title
#title("Test from subdirectory")
"#;
        std::fs::write(dir.path().join("sub/nested.typ"), sub_content).unwrap();

        let generator = PdfGenerator::new();
        let member = test_member();
        let result = generator.render("sub/nested.typ", dir.path(), &member);

        assert!(result.is_ok());
    }

    #[test]
    fn test_render_compilation_error() {
        let dir = TempDir::new().unwrap();
        let bad_content = r#"
#let x =
// incomplete expression
"#;
        std::fs::write(dir.path().join("bad.typ"), bad_content).unwrap();

        let generator = PdfGenerator::new();
        let member = test_member();
        let result = generator.render("bad.typ", dir.path(), &member);

        assert!(matches!(result, Err(TemplateError::RenderError(_))));
    }

    #[test]
    fn test_render_nonexistent_template() {
        let dir = TempDir::new().unwrap();
        let generator = PdfGenerator::new();
        let member = test_member();
        let result = generator.render("nonexistent.typ", dir.path(), &member);

        assert!(matches!(result, Err(TemplateError::NotFound)));
    }

    #[test]
    fn test_render_default_templates() {
        let dir = TempDir::new().unwrap();

        // Write the default templates
        std::fs::write(
            dir.path().join("_layout.typ"),
            include_bytes!("../../templates/defaults/_layout.typ").as_slice(),
        )
        .unwrap();
        std::fs::write(
            dir.path().join("join_confirmation.typ"),
            include_bytes!("../../templates/defaults/join_confirmation.typ").as_slice(),
        )
        .unwrap();

        let generator = PdfGenerator::new();
        let member = test_member();
        let result = generator.render("join_confirmation.typ", dir.path(), &member);

        assert!(
            result.is_ok(),
            "Failed to render default template: {:?}",
            result.err()
        );
        let pdf = result.unwrap();
        assert!(pdf.starts_with(b"%PDF"));
    }

    // --- Phase 11 (EXPO-02) render_repayment_list tests ----------------------
    //
    // REVISION-Fix W5: `CARGO_MANIFEST_DIR` macht die Template-Pfade
    // deterministisch unabhaengig vom Cargo-Working-Dir (Workspace-Root vs.
    // Crate-Dir).
    // REVISION-Fix B1 + W6: D-04 wortwoertlich mit ORIGINAL-Umlaut
    // "Anteilsrückzahlung" (ü, NICHT ue). D-05: keine ASCII-Sanitization.
    // Member-Name "Hans Müller" verifiziert UTF-8-Render-Path durch Typst.

    fn test_repayment_phase() -> RepaymentPhaseEntity {
        let now = time::OffsetDateTime::now_utc();
        let created = time::PrimitiveDateTime::new(now.date(), now.time());
        RepaymentPhaseEntity {
            id: Uuid::new_v4(),
            fiscal_year: 2026,
            share_value: 12000, // 120 EUR in cents
            status: genossi_dao::repayment_phase::RepaymentPhaseStatus::Open,
            opened_at: None,
            closed_at: None,
            created,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    #[test]
    fn test_render_repayment_list_with_empty_rows() {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let template_base = TempDir::new().unwrap();
        std::fs::copy(
            manifest.join("../templates/defaults/_layout.typ"),
            template_base.path().join("_layout.typ"),
        )
        .unwrap();
        std::fs::copy(
            manifest.join("../templates/defaults/auszahlungsliste.typ"),
            template_base.path().join("auszahlungsliste.typ"),
        )
        .unwrap();

        let generator = PdfGenerator::new();
        let phase = test_repayment_phase();
        let rows: Vec<RepaymentExportRow> = Vec::new();
        let res = generator.render_repayment_list(
            "auszahlungsliste.typ",
            template_base.path(),
            &phase,
            &rows,
        );
        assert!(res.is_ok(), "render failed: {:?}", res.err());
        let bytes = res.unwrap();
        assert!(bytes.starts_with(b"%PDF-"), "PDF magic bytes missing");
    }

    #[test]
    fn test_render_repayment_list_with_two_rows() {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let template_base = TempDir::new().unwrap();
        std::fs::copy(
            manifest.join("../templates/defaults/_layout.typ"),
            template_base.path().join("_layout.typ"),
        )
        .unwrap();
        std::fs::copy(
            manifest.join("../templates/defaults/auszahlungsliste.typ"),
            template_base.path().join("auszahlungsliste.typ"),
        )
        .unwrap();

        let generator = PdfGenerator::new();
        let phase = test_repayment_phase();
        // REVISION-Fix B1 + W6: D-04 wortwoertlich mit ORIGINAL-Umlaut.
        let rows = vec![
            RepaymentExportRow {
                member_number: 1234,
                name: "Hans Müller".to_string(),
                iban: "DE89370400440532013000".to_string(),
                share_count: 1,
                amount_str: "120,00".to_string(),
                purpose: "Anteilsrückzahlung GJ 2026 1234 Hans Müller".to_string(),
            },
            RepaymentExportRow {
                member_number: 5678,
                name: "Erika Beispiel".to_string(),
                iban: String::new(), // D-07: IBAN-NULL als leerer String
                share_count: 2,
                amount_str: "240,00".to_string(),
                purpose: "Anteilsrückzahlung GJ 2026 5678 Erika Beispiel".to_string(),
            },
        ];

        // Sanity: purpose-Strings enthalten ORIGINAL-Umlaut (D-04 wortwoertlich).
        assert!(
            rows[0].purpose.contains('ü'),
            "D-04 violated: purpose must contain 'ü' from 'Anteilsrückzahlung'"
        );
        assert_eq!(
            rows[0].purpose,
            "Anteilsrückzahlung GJ 2026 1234 Hans Müller"
        );

        let res = generator.render_repayment_list(
            "auszahlungsliste.typ",
            template_base.path(),
            &phase,
            &rows,
        );
        assert!(res.is_ok(), "render failed: {:?}", res.err());
        let bytes = res.unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
        assert!(bytes.len() > 1000, "PDF too small ({} bytes)", bytes.len());
    }

    #[test]
    fn test_package_cache_package_dir() {
        let cache = PackageCache {
            cache_dir: PathBuf::from("/tmp/typst-cache"),
            downloaded: Mutex::new(HashSet::new()),
        };
        let pkg = test_package_spec();
        let dir = cache.package_dir(&pkg);
        assert_eq!(dir, PathBuf::from("/tmp/typst-cache/preview/example/1.0.0"));
    }

    #[test]
    fn test_resolve_path_with_package() {
        let dir = TempDir::new().unwrap();
        let cache = PackageCache {
            cache_dir: PathBuf::from("/tmp/typst-cache"),
            downloaded: Mutex::new(HashSet::new()),
        };
        let generator = PdfGenerator::new();
        let inputs = Dict::new();
        let world = TemplateWorld::new(
            "// empty",
            "test.typ",
            dir.path().to_path_buf(),
            inputs,
            &generator.fonts,
            &generator.book,
            &cache,
        );

        // Local file resolves to template_base
        let local_id = FileId::new(None, VirtualPath::new("test.typ"));
        let local_path = world.resolve_path(local_id);
        assert_eq!(local_path, dir.path().join("test.typ"));

        // Package file resolves to cache dir
        let pkg = test_package_spec();
        let pkg_id = FileId::new(Some(pkg), VirtualPath::new("lib.typ"));
        let pkg_path = world.resolve_path(pkg_id);
        assert_eq!(
            pkg_path,
            PathBuf::from("/tmp/typst-cache/preview/example/1.0.0/lib.typ")
        );
    }

    #[test]
    fn test_render_with_local_import_still_works() {
        // Regression test: local imports must continue to work
        let dir = TempDir::new().unwrap();

        let layout_content = r#"
#let greet(name) = {
  text(weight: "bold")[Hello, #name!]
}
"#;
        std::fs::write(dir.path().join("_layout.typ"), layout_content).unwrap();

        let main_content = r#"
#import "_layout.typ": greet
#let member = json.decode(sys.inputs.at("member"))
#greet(member.first_name)
"#;
        std::fs::write(dir.path().join("main.typ"), main_content).unwrap();

        let generator = PdfGenerator::new();
        let member = test_member();
        let result = generator.render("main.typ", dir.path(), &member);

        assert!(
            result.is_ok(),
            "Local import regression: {:?}",
            result.err()
        );
    }

    #[test]
    #[ignore] // Requires network access; fails in sandboxed builds (e.g. nix)
    fn test_render_with_package_import() {
        // Integration test: downloads a real (small) package from the registry
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();

        let template_content = r#"
#import "@preview/oxifmt:0.2.1": strfmt
#let member = json.decode(sys.inputs.at("member"))
#strfmt("Hello {}!", member.first_name)
"#;
        std::fs::write(dir.path().join("test.typ"), template_content).unwrap();

        let generator = PdfGenerator {
            fonts: generator_fonts(),
            book: generator_book(),
            package_cache: PackageCache {
                cache_dir: cache_dir.path().to_path_buf(),
                downloaded: Mutex::new(HashSet::new()),
            },
        };
        let member = test_member();
        let result = generator.render("test.typ", dir.path(), &member);

        assert!(result.is_ok(), "Package import failed: {:?}", result.err());
        let pdf = result.unwrap();
        assert!(pdf.starts_with(b"%PDF"));

        // Verify package was cached
        assert!(cache_dir.path().join("preview/oxifmt/0.2.1").exists());
    }

    #[test]
    fn test_package_not_found() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();

        let template_content = r#"
#import "@preview/this-package-does-not-exist-xyz:0.0.1": foo
foo
"#;
        std::fs::write(dir.path().join("test.typ"), template_content).unwrap();

        let generator = PdfGenerator {
            fonts: generator_fonts(),
            book: generator_book(),
            package_cache: PackageCache {
                cache_dir: cache_dir.path().to_path_buf(),
                downloaded: Mutex::new(HashSet::new()),
            },
        };
        let member = test_member();
        let result = generator.render("test.typ", dir.path(), &member);

        assert!(matches!(result, Err(TemplateError::RenderError(_))));
    }

    fn generator_fonts() -> Vec<Font> {
        let mut fonts = Vec::new();
        for data in EMBEDDED_FONTS {
            let bytes = Bytes::new(data.to_vec());
            for font in Font::iter(bytes) {
                fonts.push(font);
            }
        }
        fonts
    }

    fn generator_book() -> LazyHash<FontBook> {
        let fonts = generator_fonts();
        LazyHash::new(FontBook::from_fonts(fonts.iter()))
    }

    // --- Phase 13 D-13-01 build_inputs_repayment_letter(_bundle) tests --------
    //
    // Verifizieren JSON-Shape fuer Single-Letter und Bundle Inputs.
    // Pitfall #5: bank_account=None → JSON null (Template hat #if-Switch).

    fn sample_member_with_iban() -> MemberEntity {
        let now = time::OffsetDateTime::now_utc();
        MemberEntity {
            id: Uuid::new_v4(),
            member_number: 1234,
            first_name: Arc::from("Hans"),
            last_name: Arc::from("Müller"),
            salutation: Some(genossi_dao::member::Salutation::Herr),
            title: Some(Arc::from("Dr.")),
            email: Some(Arc::from("hans@example.com")),
            company: None,
            comment: None,
            street: Some(Arc::from("Musterstraße")),
            house_number: Some(Arc::from("42")),
            postal_code: Some(Arc::from("12345")),
            city: Some(Arc::from("München")),
            join_date: time::Date::from_calendar_date(2024, time::Month::January, 15).unwrap(),
            shares_at_joining: 3,
            current_shares: 5,
            current_balance: 15000,
            action_count: 2,
            migrated: false,
            exit_date: None,
            bank_account: Some(Arc::from("DE89370400440532013000")),
            status: genossi_dao::member::MemberStatus::Normal,
            created: time::PrimitiveDateTime::new(now.date(), now.time()),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn sample_member_without_iban() -> MemberEntity {
        MemberEntity {
            member_number: 5678,
            first_name: Arc::from("Erika"),
            last_name: Arc::from("Beispiel"),
            salutation: Some(genossi_dao::member::Salutation::Frau),
            title: None,
            bank_account: None,
            ..sample_member_with_iban()
        }
    }

    fn sample_ctx(share_count: i32, payout_amount: &str, fiscal_year: i32) -> RepaymentContext {
        RepaymentContext {
            share_count,
            payout_amount: payout_amount.to_string(),
            fiscal_year,
        }
    }

    /// Extracts the JSON-string value behind a given Dict key. Panics if the
    /// key is missing or the value is not a string.
    fn extract_str_input(dict: &Dict, key: &str) -> String {
        match dict.get(&Str::from(key)).expect("key missing in dict") {
            Value::Str(s) => s.to_string(),
            _ => panic!("expected string value for key {}", key),
        }
    }

    #[test]
    fn test_build_inputs_repayment_letter_has_three_top_level_keys() {
        let phase = test_repayment_phase();
        let member = sample_member_with_iban();
        let ctx = sample_ctx(3, "360,00", 2025);
        let dict = build_inputs_repayment_letter(&phase, &member, &ctx);

        let keys: Vec<String> = dict
            .iter()
            .map(|(k, _)| k.as_str().to_string())
            .collect();
        assert!(keys.contains(&"member".to_string()), "missing 'member' key");
        assert!(
            keys.contains(&"repayment".to_string()),
            "missing 'repayment' key"
        );
        assert!(keys.contains(&"today".to_string()), "missing 'today' key");
    }

    #[test]
    fn test_build_inputs_repayment_letter_member_has_ten_fields() {
        let phase = test_repayment_phase();
        let member = sample_member_with_iban();
        let ctx = sample_ctx(3, "360,00", 2025);
        let dict = build_inputs_repayment_letter(&phase, &member, &ctx);
        let member_json: serde_json::Value =
            serde_json::from_str(extract_str_input(&dict, "member").as_str()).unwrap();

        assert_eq!(member_json["member_number"], 1234);
        assert_eq!(member_json["first_name"], "Hans");
        assert_eq!(member_json["last_name"], "Müller");
        assert_eq!(member_json["salutation"], "Herr");
        assert_eq!(member_json["title"], "Dr.");
        assert_eq!(member_json["street"], "Musterstraße");
        assert_eq!(member_json["house_number"], "42");
        assert_eq!(member_json["postal_code"], "12345");
        assert_eq!(member_json["city"], "München");
        assert_eq!(member_json["bank_account"], "DE89370400440532013000");
    }

    #[test]
    fn test_build_inputs_repayment_letter_bank_account_null_is_json_null() {
        // Pitfall #5: bank_account=None muss als JSON null serialisiert werden.
        let phase = test_repayment_phase();
        let member = sample_member_without_iban();
        let ctx = sample_ctx(3, "360,00", 2025);
        let dict = build_inputs_repayment_letter(&phase, &member, &ctx);
        let member_json: serde_json::Value =
            serde_json::from_str(extract_str_input(&dict, "member").as_str()).unwrap();
        assert_eq!(member_json["bank_account"], serde_json::Value::Null);
        assert_eq!(member_json["title"], serde_json::Value::Null);
    }

    #[test]
    fn test_build_inputs_repayment_letter_repayment_keys() {
        let phase = test_repayment_phase();
        let member = sample_member_with_iban();
        let ctx = sample_ctx(5, "600,00", 2025);
        let dict = build_inputs_repayment_letter(&phase, &member, &ctx);
        let repayment_json: serde_json::Value =
            serde_json::from_str(extract_str_input(&dict, "repayment").as_str()).unwrap();
        assert_eq!(repayment_json["share_count"], 5);
        assert_eq!(repayment_json["payout_amount"], "600,00");
        assert_eq!(repayment_json["fiscal_year"], 2025);
    }

    #[test]
    fn test_build_inputs_repayment_letter_today_is_iso_date() {
        let phase = test_repayment_phase();
        let member = sample_member_with_iban();
        let ctx = sample_ctx(1, "120,00", 2025);
        let dict = build_inputs_repayment_letter(&phase, &member, &ctx);
        let today = extract_str_input(&dict, "today");
        // ISO 8601 date: YYYY-MM-DD (10 chars).
        assert_eq!(today.len(), 10, "today is not ISO date: {}", today);
        assert!(
            today.chars().nth(4) == Some('-') && today.chars().nth(7) == Some('-'),
            "today is not in YYYY-MM-DD format: {}",
            today
        );
    }

    #[test]
    fn test_build_inputs_bundle_recipients_is_array() {
        let phase = test_repayment_phase();
        let recipients = vec![
            (sample_member_with_iban(), sample_ctx(3, "360,00", 2025)),
            (sample_member_without_iban(), sample_ctx(2, "240,00", 2025)),
        ];
        let dict = build_inputs_repayment_letters_bundle(&phase, &recipients);
        let recipients_json: serde_json::Value =
            serde_json::from_str(extract_str_input(&dict, "recipients").as_str()).unwrap();
        assert!(recipients_json.is_array(), "recipients is not a JSON array");
        assert_eq!(recipients_json.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_build_inputs_bundle_recipient_has_member_and_repayment_subkeys() {
        let phase = test_repayment_phase();
        let recipients = vec![(sample_member_with_iban(), sample_ctx(3, "360,00", 2025))];
        let dict = build_inputs_repayment_letters_bundle(&phase, &recipients);
        let recipients_json: serde_json::Value =
            serde_json::from_str(extract_str_input(&dict, "recipients").as_str()).unwrap();
        let first = &recipients_json.as_array().unwrap()[0];
        assert!(first["member"].is_object(), "missing member subkey");
        assert!(first["repayment"].is_object(), "missing repayment subkey");
        assert_eq!(first["member"]["member_number"], 1234);
        assert_eq!(first["repayment"]["share_count"], 3);
    }

    #[test]
    fn test_build_inputs_bundle_meta_has_recipient_count() {
        let phase = test_repayment_phase();
        let recipients = vec![(sample_member_with_iban(), sample_ctx(3, "360,00", 2025))];
        let dict = build_inputs_repayment_letters_bundle(&phase, &recipients);
        let meta_json: serde_json::Value =
            serde_json::from_str(extract_str_input(&dict, "meta").as_str()).unwrap();
        assert_eq!(meta_json["recipient_count"], 1);
        assert_eq!(meta_json["fiscal_year"], phase.fiscal_year);
    }

    #[test]
    fn test_build_inputs_bundle_bank_account_null_in_recipient() {
        // Pitfall #5 im Bundle-Pfad: NULL-IBAN bleibt JSON null.
        let phase = test_repayment_phase();
        let recipients = vec![(sample_member_without_iban(), sample_ctx(2, "240,00", 2025))];
        let dict = build_inputs_repayment_letters_bundle(&phase, &recipients);
        let recipients_json: serde_json::Value =
            serde_json::from_str(extract_str_input(&dict, "recipients").as_str()).unwrap();
        let first = &recipients_json.as_array().unwrap()[0];
        assert_eq!(first["member"]["bank_account"], serde_json::Value::Null);
    }
}
