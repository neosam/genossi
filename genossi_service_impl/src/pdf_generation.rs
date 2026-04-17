use genossi_service::application::Application;
use genossi_service::member::Member;
use genossi_service::template::TemplateError;
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

pub struct PackageCache {
    cache_dir: PathBuf,
    downloaded: Mutex<HashSet<String>>,
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
}
