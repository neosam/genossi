use genossi_service::template::{FileTreeEntry, TemplateError};
use std::path::{Path, PathBuf};
use std::sync::Arc;

struct DefaultTemplate {
    path: &'static str,
    content: &'static [u8],
}

const DEFAULT_TEMPLATES: &[DefaultTemplate] = &[
    DefaultTemplate {
        path: "_layout.typ",
        content: include_bytes!("../../templates/defaults/_layout.typ"),
    },
    DefaultTemplate {
        path: "join_confirmation.typ",
        content: include_bytes!("../../templates/defaults/join_confirmation.typ"),
    },
    // Phase 6 (D-04, D-08, D-10): Teilnehmerlisten-Export PDF template.
    // Required by AttendanceExportServiceImpl::export(ExportFormat::Pdf).
    // Without this entry the PDF-Export branch fails with a "template not
    // found" InternalError on a fresh installation.
    DefaultTemplate {
        path: "teilnehmerliste.typ",
        content: include_bytes!("../../templates/defaults/teilnehmerliste.typ"),
    },
    // Phase 11 (EXPO-01..03): Auszahlungslisten-Export PDF template.
    // Required by RepaymentExportServiceImpl::export(ExportFormat::Pdf) — Plan 11.03.
    // Without this entry the PDF-Export branch fails with a "template not
    // found" InternalError on a fresh installation (Pitfall #1 RESEARCH §Common Pitfalls).
    DefaultTemplate {
        path: "auszahlungsliste.typ",
        content: include_bytes!("../../templates/defaults/auszahlungsliste.typ"),
    },
    // Phase 13 D-13-05: RepaymentLetter Single-Letter Default-Template.
    // UI-editierbar via /templates-Editor; DEFAULT_TEMPLATES liefert nur den Initial-Wert.
    // RESEARCH §Pitfall #1: ohne diesen Eintrag faellt das Letter-Service
    // mit "template not found"-InternalError beim ersten Render.
    DefaultTemplate {
        path: "auszahlungs_anschreiben.typ",
        content: include_bytes!("../../templates/defaults/auszahlungs_anschreiben.typ"),
    },
    // Phase 13 D-13-01: RepaymentLetter Bundle-Wrapper-Template.
    // Importiert render-letter aus auszahlungs_anschreiben.typ (Single-Source-of-Truth)
    // und iteriert ueber recipients[] mit #pagebreak() dazwischen.
    DefaultTemplate {
        path: "auszahlungs_anschreiben_bundle.typ",
        content: include_bytes!("../../templates/defaults/auszahlungs_anschreiben_bundle.typ"),
    },
];

pub struct TemplateStorage {
    base_path: PathBuf,
}

impl TemplateStorage {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    pub fn from_env() -> Self {
        let path = std::env::var("TEMPLATE_PATH").unwrap_or_else(|_| "./templates".into());
        Self::new(PathBuf::from(path))
    }

    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    fn validate_path(&self, relative_path: &str) -> Result<PathBuf, TemplateError> {
        // Reject absolute paths
        if relative_path.starts_with('/') || relative_path.starts_with('\\') {
            return Err(TemplateError::PathTraversal);
        }

        // Reject path traversal
        for component in Path::new(relative_path).components() {
            if let std::path::Component::ParentDir = component {
                return Err(TemplateError::PathTraversal);
            }
        }

        let full_path = self.base_path.join(relative_path);

        // Verify the resolved path is within base_path
        let canonical_base = self
            .base_path
            .canonicalize()
            .map_err(|e| TemplateError::IoError(Arc::from(e.to_string())))?;

        // For new files that don't exist yet, check the parent
        if full_path.exists() {
            let canonical_full = full_path
                .canonicalize()
                .map_err(|e| TemplateError::IoError(Arc::from(e.to_string())))?;
            if !canonical_full.starts_with(&canonical_base) {
                return Err(TemplateError::PathTraversal);
            }
        } else {
            // Check that the parent exists and is within base
            if let Some(parent) = full_path.parent() {
                if parent.exists() {
                    let canonical_parent = parent
                        .canonicalize()
                        .map_err(|e| TemplateError::IoError(Arc::from(e.to_string())))?;
                    if !canonical_parent.starts_with(&canonical_base) {
                        return Err(TemplateError::PathTraversal);
                    }
                }
            }
        }

        Ok(full_path)
    }

    /// Provision default templates on startup. Only writes files that don't exist.
    pub async fn provision_defaults(&self) -> Result<(), TemplateError> {
        tokio::fs::create_dir_all(&self.base_path)
            .await
            .map_err(|e| TemplateError::IoError(Arc::from(e.to_string())))?;

        for template in DEFAULT_TEMPLATES {
            let target = self.base_path.join(template.path);
            if !target.exists() {
                if let Some(parent) = target.parent() {
                    tokio::fs::create_dir_all(parent)
                        .await
                        .map_err(|e| TemplateError::IoError(Arc::from(e.to_string())))?;
                }
                tokio::fs::write(&target, template.content)
                    .await
                    .map_err(|e| TemplateError::IoError(Arc::from(e.to_string())))?;
                tracing::info!("Provisioned default template: {}", template.path);
            }
        }

        Ok(())
    }

    /// List all templates as a recursive file tree.
    pub async fn list_tree(&self) -> Result<Vec<FileTreeEntry>, TemplateError> {
        if !self.base_path.exists() {
            return Ok(Vec::new());
        }
        self.read_dir_recursive(&self.base_path, "").await
    }

    fn read_dir_recursive(
        &self,
        dir: &Path,
        prefix: &str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<FileTreeEntry>, TemplateError>> + Send + '_,
        >,
    > {
        let dir = dir.to_path_buf();
        let prefix = prefix.to_string();
        Box::pin(async move {
            let mut entries = Vec::new();
            let mut read_dir = tokio::fs::read_dir(&dir)
                .await
                .map_err(|e| TemplateError::IoError(Arc::from(e.to_string())))?;

            let mut items = Vec::new();
            while let Some(entry) = read_dir
                .next_entry()
                .await
                .map_err(|e| TemplateError::IoError(Arc::from(e.to_string())))?
            {
                items.push(entry);
            }

            // Sort by name
            items.sort_by_key(|e| e.file_name());

            for entry in items {
                let file_name = entry.file_name();
                let name = file_name.to_string_lossy();
                let relative_path = if prefix.is_empty() {
                    name.to_string()
                } else {
                    format!("{}/{}", prefix, name)
                };

                let file_type = entry
                    .file_type()
                    .await
                    .map_err(|e| TemplateError::IoError(Arc::from(e.to_string())))?;

                if file_type.is_dir() {
                    let children = self
                        .read_dir_recursive(&entry.path(), &relative_path)
                        .await?;
                    entries.push(FileTreeEntry::Directory {
                        name: name.to_string(),
                        path: relative_path.clone(),
                        children,
                    });
                } else if file_type.is_file() {
                    entries.push(FileTreeEntry::File {
                        name: name.to_string(),
                        path: relative_path.clone(),
                    });
                }
            }

            Ok(entries)
        })
    }

    /// Read a template file's content.
    pub async fn read_file(&self, relative_path: &str) -> Result<String, TemplateError> {
        let full_path = self.validate_path(relative_path)?;
        if !full_path.exists() || !full_path.is_file() {
            return Err(TemplateError::NotFound);
        }
        tokio::fs::read_to_string(&full_path)
            .await
            .map_err(|e| TemplateError::IoError(Arc::from(e.to_string())))
    }

    /// Create or update a template file. Creates parent directories as needed.
    pub async fn write_file(
        &self,
        relative_path: &str,
        content: &str,
    ) -> Result<(), TemplateError> {
        let full_path = self.validate_path(relative_path)?;
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| TemplateError::IoError(Arc::from(e.to_string())))?;
        }
        tokio::fs::write(&full_path, content)
            .await
            .map_err(|e| TemplateError::IoError(Arc::from(e.to_string())))
    }

    /// Read a template file's raw bytes.
    pub async fn read_file_bytes(&self, relative_path: &str) -> Result<Vec<u8>, TemplateError> {
        let full_path = self.validate_path(relative_path)?;
        if !full_path.exists() || !full_path.is_file() {
            return Err(TemplateError::NotFound);
        }
        tokio::fs::read(&full_path)
            .await
            .map_err(|e| TemplateError::IoError(Arc::from(e.to_string())))
    }

    /// Create or update a template file with binary content. Creates parent directories as needed.
    pub async fn write_file_bytes(
        &self,
        relative_path: &str,
        content: &[u8],
    ) -> Result<(), TemplateError> {
        let full_path = self.validate_path(relative_path)?;
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| TemplateError::IoError(Arc::from(e.to_string())))?;
        }
        tokio::fs::write(&full_path, content)
            .await
            .map_err(|e| TemplateError::IoError(Arc::from(e.to_string())))
    }

    /// Create an empty directory.
    pub async fn create_directory(&self, relative_path: &str) -> Result<(), TemplateError> {
        let full_path = self.validate_path(relative_path)?;
        tokio::fs::create_dir_all(&full_path)
            .await
            .map_err(|e| TemplateError::IoError(Arc::from(e.to_string())))
    }

    /// Delete a file or empty directory.
    pub async fn delete(&self, relative_path: &str) -> Result<(), TemplateError> {
        let full_path = self.validate_path(relative_path)?;
        if !full_path.exists() {
            return Err(TemplateError::NotFound);
        }

        if full_path.is_dir() {
            // Check if directory is empty
            let mut read_dir = tokio::fs::read_dir(&full_path)
                .await
                .map_err(|e| TemplateError::IoError(Arc::from(e.to_string())))?;
            if read_dir
                .next_entry()
                .await
                .map_err(|e| TemplateError::IoError(Arc::from(e.to_string())))?
                .is_some()
            {
                return Err(TemplateError::DirectoryNotEmpty);
            }
            tokio::fs::remove_dir(&full_path)
                .await
                .map_err(|e| TemplateError::IoError(Arc::from(e.to_string())))
        } else {
            tokio::fs::remove_file(&full_path)
                .await
                .map_err(|e| TemplateError::IoError(Arc::from(e.to_string())))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_storage() -> (TempDir, TemplateStorage) {
        let dir = TempDir::new().unwrap();
        let storage = TemplateStorage::new(dir.path().to_path_buf());
        (dir, storage)
    }

    #[tokio::test]
    async fn test_provision_defaults() {
        let (_dir, storage) = test_storage();
        storage.provision_defaults().await.unwrap();

        // Check that default files exist
        assert!(storage.base_path.join("_layout.typ").exists());
        assert!(storage.base_path.join("join_confirmation.typ").exists());
    }

    #[tokio::test]
    async fn test_provision_defaults_does_not_overwrite() {
        let (_dir, storage) = test_storage();

        // Create a custom version first
        tokio::fs::create_dir_all(&storage.base_path).await.unwrap();
        tokio::fs::write(storage.base_path.join("_layout.typ"), "custom content")
            .await
            .unwrap();

        storage.provision_defaults().await.unwrap();

        // Custom content should be preserved
        let content = tokio::fs::read_to_string(storage.base_path.join("_layout.typ"))
            .await
            .unwrap();
        assert_eq!(content, "custom content");
    }

    #[tokio::test]
    async fn test_write_and_read_file() {
        let (_dir, storage) = test_storage();
        tokio::fs::create_dir_all(&storage.base_path).await.unwrap();

        storage.write_file("test.typ", "hello world").await.unwrap();
        let content = storage.read_file("test.typ").await.unwrap();
        assert_eq!(content, "hello world");
    }

    #[tokio::test]
    async fn test_write_file_creates_parent_dirs() {
        let (_dir, storage) = test_storage();
        tokio::fs::create_dir_all(&storage.base_path).await.unwrap();

        storage
            .write_file("sub/dir/test.typ", "nested content")
            .await
            .unwrap();
        let content = storage.read_file("sub/dir/test.typ").await.unwrap();
        assert_eq!(content, "nested content");
    }

    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let (_dir, storage) = test_storage();
        tokio::fs::create_dir_all(&storage.base_path).await.unwrap();

        let result = storage.read_file("nonexistent.typ").await;
        assert!(matches!(result, Err(TemplateError::NotFound)));
    }

    #[tokio::test]
    async fn test_delete_file() {
        let (_dir, storage) = test_storage();
        tokio::fs::create_dir_all(&storage.base_path).await.unwrap();

        storage.write_file("to_delete.typ", "bye").await.unwrap();
        storage.delete("to_delete.typ").await.unwrap();

        let result = storage.read_file("to_delete.typ").await;
        assert!(matches!(result, Err(TemplateError::NotFound)));
    }

    #[tokio::test]
    async fn test_delete_empty_directory() {
        let (_dir, storage) = test_storage();
        tokio::fs::create_dir_all(&storage.base_path).await.unwrap();

        storage.create_directory("empty_dir").await.unwrap();
        storage.delete("empty_dir").await.unwrap();

        assert!(!storage.base_path.join("empty_dir").exists());
    }

    #[tokio::test]
    async fn test_delete_nonempty_directory() {
        let (_dir, storage) = test_storage();
        tokio::fs::create_dir_all(&storage.base_path).await.unwrap();

        storage
            .write_file("nonempty/file.typ", "content")
            .await
            .unwrap();
        let result = storage.delete("nonempty").await;
        assert!(matches!(result, Err(TemplateError::DirectoryNotEmpty)));
    }

    #[tokio::test]
    async fn test_delete_nonexistent() {
        let (_dir, storage) = test_storage();
        tokio::fs::create_dir_all(&storage.base_path).await.unwrap();

        let result = storage.delete("nope.typ").await;
        assert!(matches!(result, Err(TemplateError::NotFound)));
    }

    #[tokio::test]
    async fn test_path_traversal_rejected() {
        let (_dir, storage) = test_storage();
        tokio::fs::create_dir_all(&storage.base_path).await.unwrap();

        let result = storage.read_file("../../etc/passwd").await;
        assert!(matches!(result, Err(TemplateError::PathTraversal)));

        let result = storage.read_file("/etc/passwd").await;
        assert!(matches!(result, Err(TemplateError::PathTraversal)));
    }

    #[tokio::test]
    async fn test_list_tree() {
        let (_dir, storage) = test_storage();
        storage.provision_defaults().await.unwrap();
        storage
            .write_file("sub/nested.typ", "nested")
            .await
            .unwrap();

        let tree = storage.list_tree().await.unwrap();

        // Should contain _layout.typ, join_confirmation.typ, and sub/ directory
        assert!(tree.len() >= 3);

        // Check that directory has children
        let sub_dir = tree
            .iter()
            .find(|e| matches!(e, FileTreeEntry::Directory { name, .. } if name == "sub"));
        assert!(sub_dir.is_some());
        if let Some(FileTreeEntry::Directory { children, .. }) = sub_dir {
            assert_eq!(children.len(), 1);
        }
    }

    #[tokio::test]
    async fn test_list_tree_empty() {
        let (_dir, storage) = test_storage();
        let tree = storage.list_tree().await.unwrap();
        assert!(tree.is_empty());
    }

    #[tokio::test]
    async fn test_create_directory() {
        let (_dir, storage) = test_storage();
        tokio::fs::create_dir_all(&storage.base_path).await.unwrap();

        storage.create_directory("new_dir").await.unwrap();
        assert!(storage.base_path.join("new_dir").is_dir());
    }

    #[tokio::test]
    async fn test_create_directory_idempotent() {
        let (_dir, storage) = test_storage();
        tokio::fs::create_dir_all(&storage.base_path).await.unwrap();

        storage.create_directory("existing").await.unwrap();
        storage.create_directory("existing").await.unwrap();
        assert!(storage.base_path.join("existing").is_dir());
    }

    #[tokio::test]
    async fn test_write_and_read_file_bytes() {
        let (_dir, storage) = test_storage();
        tokio::fs::create_dir_all(&storage.base_path).await.unwrap();

        // PNG header bytes (not valid text)
        let png_header: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        storage
            .write_file_bytes("logo.png", png_header)
            .await
            .unwrap();

        // Verify bytes were written correctly
        let written = tokio::fs::read(storage.base_path.join("logo.png"))
            .await
            .unwrap();
        assert_eq!(written, png_header);
    }

    #[tokio::test]
    async fn test_write_file_bytes_creates_parent_dirs() {
        let (_dir, storage) = test_storage();
        tokio::fs::create_dir_all(&storage.base_path).await.unwrap();

        let data = &[0xFF, 0xD8, 0xFF, 0xE0]; // JPEG header
        storage
            .write_file_bytes("images/photo.jpg", data)
            .await
            .unwrap();

        let written = tokio::fs::read(storage.base_path.join("images/photo.jpg"))
            .await
            .unwrap();
        assert_eq!(written, data);
    }

    // -------------------------------------------------------------------------
    // Phase 13 D-13-01 / D-13-05 / D-13-06 / D-13-07:
    // RepaymentLetter Default-Template registriert (Single + Bundle), und
    // Single-Source-of-Truth-Vertrag: der Brief-Body lebt nur im Single-Template,
    // das Bundle importiert die `render-letter`-Funktion.
    // -------------------------------------------------------------------------

    #[test]
    fn test_default_templates_contains_auszahlungs_anschreiben_single() {
        assert!(DEFAULT_TEMPLATES
            .iter()
            .any(|t| t.path == "auszahlungs_anschreiben.typ"));
        let entry = DEFAULT_TEMPLATES
            .iter()
            .find(|t| t.path == "auszahlungs_anschreiben.typ")
            .unwrap();
        assert!(!entry.content.is_empty());
        let text = std::str::from_utf8(entry.content).expect("template is UTF-8");
        assert!(text.contains("letter-pro"), "letter-pro-Import muss vorhanden sein");
        assert!(text.contains("sys.inputs"), "sys.inputs-Pattern muss verwendet werden");
        assert!(
            text.contains("bank_account"),
            "IBAN-Switch muss vorhanden sein (D-13-06)"
        );
        assert!(
            text.contains("Carolin Weidmann"),
            "Vorstands-Signatur (D-13-06 Baustein 4)"
        );
        assert!(
            text.contains("render-letter"),
            "render-letter-Funktion muss exportiert sein (Single-Source-of-Truth)"
        );
        assert!(
            !text.contains("Verwendungszweck"),
            "D-13-07: KEIN SEPA-Verwendungszweck"
        );
    }

    #[test]
    fn test_default_templates_contains_auszahlungs_anschreiben_bundle() {
        assert!(DEFAULT_TEMPLATES
            .iter()
            .any(|t| t.path == "auszahlungs_anschreiben_bundle.typ"));
        let entry = DEFAULT_TEMPLATES
            .iter()
            .find(|t| t.path == "auszahlungs_anschreiben_bundle.typ")
            .unwrap();
        assert!(!entry.content.is_empty());
        let text = std::str::from_utf8(entry.content).expect("template is UTF-8");
        assert!(
            text.contains("#import \"auszahlungs_anschreiben.typ\""),
            "Bundle MUSS render-letter aus Single-Template importieren (Single-Source-of-Truth)"
        );
        assert!(
            text.contains("render-letter"),
            "Bundle ruft render-letter auf"
        );
        assert!(
            text.contains("pagebreak"),
            "Bundle MUSS #pagebreak() zwischen Briefen setzen (D-13-01)"
        );
        assert!(
            text.contains("recipients"),
            "Bundle iteriert ueber recipients-Array"
        );
        // Drift-Schutz: Brief-Body-Strings duerfen NICHT im Bundle dupliziert sein.
        assert!(
            !text.contains("Carolin Weidmann"),
            "Drift-Schutz: Vorstands-Signatur lebt NUR im Single-Template (Single-Source-of-Truth)"
        );
        assert!(
            !text.contains("Mitgliedsnummer"),
            "Drift-Schutz: Reference-Block-Strings leben NUR im Single-Template"
        );
    }
}
