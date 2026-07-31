use std::{
    fs,
    path::{Path, PathBuf},
};

use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum DocumentStorageError {
    #[error("could not access RADcite source file")]
    Io(#[from] std::io::Error),
}

pub fn managed_source_path(
    data_dir: &Path,
    project_id: Uuid,
    document_id: Uuid,
    original_filename: &str,
) -> PathBuf {
    let filename = Path::new(original_filename)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("source");
    let safe_filename = filename
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    let safe_filename = if safe_filename.is_empty() {
        "source".to_string()
    } else {
        safe_filename
    };

    data_dir
        .join("documents")
        .join(project_id.to_string())
        .join(format!("{document_id}-{safe_filename}"))
}

pub fn store_source(
    data_dir: &Path,
    project_id: Uuid,
    document_id: Uuid,
    source_path: &Path,
    original_filename: &str,
) -> Result<PathBuf, DocumentStorageError> {
    let source_metadata = fs::metadata(source_path)?;
    if !source_metadata.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "RADcite source path is not a file",
        )
        .into());
    }

    let destination = managed_source_path(data_dir, project_id, document_id, original_filename);
    if source_path != destination {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source_path, &destination)?;
    }

    Ok(destination)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use uuid::Uuid;

    use super::{DocumentStorageError, managed_source_path, store_source};

    #[test]
    fn managed_source_path_keeps_source_inside_project_directory() {
        let data_dir =
            std::env::temp_dir().join(format!("radsuite-source-path-{}", Uuid::new_v4()));
        let project_id = Uuid::new_v4();
        let document_id = Uuid::new_v4();
        let expected_parent = data_dir.join("documents").join(project_id.to_string());

        let destination = managed_source_path(
            &data_dir,
            project_id,
            document_id,
            "../../course readings.docx",
        );

        assert_eq!(destination.parent(), Some(expected_parent.as_path()));
        assert_eq!(
            destination.extension().and_then(|value| value.to_str()),
            Some("docx")
        );
        assert!(!destination.to_string_lossy().contains(".."));
    }

    #[test]
    fn store_source_copies_the_selected_file_to_managed_storage() {
        let root = std::env::temp_dir().join(format!("radsuite-source-store-{}", Uuid::new_v4()));
        let source = root.join("incoming").join("lesson.docx");
        let data_dir = root.join("app-data");
        fs::create_dir_all(source.parent().expect("source parent")).expect("create source parent");
        fs::write(&source, b"source contents").expect("write source");

        let destination = store_source(
            &data_dir,
            project_id(),
            document_id(),
            &source,
            "lesson.docx",
        )
        .expect("store source");

        assert_eq!(
            fs::read(&destination).expect("read managed source"),
            b"source contents"
        );
        assert!(destination.starts_with(data_dir.join("documents")));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn store_source_reports_missing_input_without_creating_destination() {
        let root = std::env::temp_dir().join(format!("radsuite-source-missing-{}", Uuid::new_v4()));
        let destination_root = root.join("app-data");
        let error = store_source(
            &destination_root,
            project_id(),
            document_id(),
            &root.join("missing.pdf"),
            "missing.pdf",
        )
        .expect_err("missing source should fail");

        assert!(matches!(error, DocumentStorageError::Io(_)));
        assert!(!destination_root.exists());
        let _ = fs::remove_dir_all(root);
    }

    fn project_id() -> Uuid {
        Uuid::new_v4()
    }

    fn document_id() -> Uuid {
        Uuid::new_v4()
    }
}
