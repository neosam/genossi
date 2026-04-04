use genossi_service::member::Member;
use genossi_service::template::TemplateError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use typst::foundations::{Bytes, Dict, Str, Value};
use typst::layout::PagedDocument;
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

pub struct PdfGenerator {
    fonts: Vec<Font>,
    book: LazyHash<FontBook>,
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

        Self { fonts, book }
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
        let format =
            time::format_description::parse("[day].[month].[year]").expect("valid format");
        member_map.insert(
            "join_date".to_string(),
            serde_json::Value::String(
                member.join_date.format(&format).unwrap_or_default(),
            ),
        );
        member_map.insert(
            "exit_date".to_string(),
            member
                .exit_date
                .map(|d| {
                    serde_json::Value::String(d.format(&format).unwrap_or_default())
                })
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

        let member_json =
            serde_json::to_string(&serde_json::Value::Object(member_map)).unwrap();
        inputs.insert(Str::from("member"), Value::Str(Str::from(member_json.as_str())));

        // Add today's date
        let today = time::OffsetDateTime::now_utc().date();
        let today_str = today.format(&format).unwrap_or_default();
        inputs.insert(Str::from("today"), Value::Str(Str::from(today_str.as_str())));

        inputs
    }
}

struct TemplateWorld<'a> {
    library: LazyHash<Library>,
    book: &'a LazyHash<FontBook>,
    fonts: &'a [Font],
    main_source: Source,
    template_base: PathBuf,
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
            source_cache: std::sync::Mutex::new(HashMap::new()),
        }
    }

    fn resolve_path(&self, id: FileId) -> PathBuf {
        let vpath = id.vpath();
        let relative = vpath.as_rootless_path();
        self.template_base.join(relative)
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
        self.source_cache
            .lock()
            .unwrap()
            .insert(id, source.clone());
        Ok(source)
    }

    fn file(&self, id: FileId) -> typst::diag::FileResult<Bytes> {
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
        typst::foundations::Datetime::from_ymd(
            now.year(),
            now.month() as u8,
            now.day(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::TempDir;
    use uuid::Uuid;

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
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2024, time::Month::January, 15).unwrap(),
                time::Time::from_hms(10, 0, 0).unwrap(),
            ),
            deleted: None,
            version: Uuid::new_v4(),
        }
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

        assert!(result.is_ok(), "Failed to render default template: {:?}", result.err());
        let pdf = result.unwrap();
        assert!(pdf.starts_with(b"%PDF"));
    }
}
