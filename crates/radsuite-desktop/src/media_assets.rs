use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

const MAX_IMAGE_BYTES: u64 = 50 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum MediaAssetError {
    #[error("presenter image does not exist or is not a regular file: {path}")]
    InvalidImage { path: PathBuf },
    #[error("presenter image must be a PNG, JPEG, or static WebP file: {path}")]
    UnsupportedImage { path: PathBuf },
    #[error("animated WebP presenter images are not supported: {path}")]
    AnimatedWebp { path: PathBuf },
    #[error("presenter image exceeds the 50 MB limit: {path}")]
    ImageTooLarge { path: PathBuf },
    #[error("project identifier is invalid: {project_id}")]
    InvalidProjectId { project_id: String },
    #[error("cleanup path is outside project media storage: {path}")]
    PathOutsideProject { path: PathBuf },
    #[error("could not prepare presenter image storage at {path}: {source}")]
    Storage {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedImage {
    path: PathBuf,
    extension: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedImage {
    path: PathBuf,
    extension: String,
    is_project_cover: bool,
}

impl ManagedImage {
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn extension(&self) -> &str {
        &self.extension
    }
    pub fn is_project_cover(&self) -> bool {
        self.is_project_cover
    }
}

#[derive(Debug)]
pub struct PendingImageCommit {
    managed: ManagedImage,
    metadata_path: Option<PathBuf>,
    backups: Vec<(PathBuf, PathBuf)>,
}

impl PendingImageCommit {
    pub fn managed(&self) -> &ManagedImage {
        &self.managed
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupFailure {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecoveryReport {
    pub removed: Vec<PathBuf>,
    pub unresolved: Vec<CleanupFailure>,
}

impl StagedImage {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn extension(&self) -> &str {
        &self.extension
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectMediaStore {
    data_dir: PathBuf,
}

impl ProjectMediaStore {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    pub fn stage_image(
        &self,
        project_id: &str,
        source: &Path,
    ) -> Result<StagedImage, MediaAssetError> {
        validate_project_id(project_id)?;
        let metadata = source
            .metadata()
            .map_err(|_| MediaAssetError::InvalidImage {
                path: source.to_path_buf(),
            })?;
        if !metadata.is_file() {
            return Err(MediaAssetError::InvalidImage {
                path: source.to_path_buf(),
            });
        }
        if metadata.len() > MAX_IMAGE_BYTES {
            return Err(MediaAssetError::ImageTooLarge {
                path: source.to_path_buf(),
            });
        }

        let extension = normalized_extension(source)?;
        if extension == "webp" && is_animated_webp(source)? {
            return Err(MediaAssetError::AnimatedWebp {
                path: source.to_path_buf(),
            });
        }

        let scratch = self.project_root(project_id).join("scratch");
        fs::create_dir_all(&scratch).map_err(|source_error| MediaAssetError::Storage {
            path: scratch.clone(),
            source: source_error,
        })?;
        let path = scratch.join(format!(".partial-{}.{}", Uuid::new_v4(), extension));
        fs::copy(source, &path).map_err(|source_error| MediaAssetError::Storage {
            path: path.clone(),
            source: source_error,
        })?;
        Ok(StagedImage { path, extension })
    }

    pub fn saved_cover(&self, project_id: &str) -> Result<Option<ManagedImage>, MediaAssetError> {
        validate_project_id(project_id)?;
        let metadata_path = self.project_root(project_id).join("cover/metadata.json");
        if !metadata_path.exists() {
            return Ok(None);
        }
        let bytes =
            fs::read(&metadata_path).map_err(|source| storage_error(&metadata_path, source))?;
        let image: ManagedImage = serde_json::from_slice(&bytes).map_err(|source| {
            storage_error(
                &metadata_path,
                io::Error::new(io::ErrorKind::InvalidData, source),
            )
        })?;
        if !image.path.is_file() {
            return Ok(None);
        }
        Ok(Some(image))
    }

    pub fn prepare_commit(
        &self,
        staged: StagedImage,
        output_id: Uuid,
        save_as_cover: bool,
    ) -> Result<PendingImageCommit, MediaAssetError> {
        let project_root = staged.path.parent().and_then(Path::parent).ok_or_else(|| {
            MediaAssetError::InvalidImage {
                path: staged.path.clone(),
            }
        })?;
        let (target, metadata_path) = if save_as_cover {
            let cover_dir = project_root.join("cover");
            (
                cover_dir.join(format!("cover.{}", staged.extension)),
                Some(cover_dir.join("metadata.json")),
            )
        } else {
            (
                project_root
                    .join("exports")
                    .join(output_id.to_string())
                    .join(format!("image.{}", staged.extension)),
                None,
            )
        };
        let parent = target.parent().expect("managed image target has a parent");
        fs::create_dir_all(parent).map_err(|source| storage_error(parent, source))?;

        let mut backups = Vec::new();
        if save_as_cover {
            for entry in fs::read_dir(parent).map_err(|source| storage_error(parent, source))? {
                let path = entry
                    .map_err(|source| storage_error(parent, source))?
                    .path();
                if path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|name| name.starts_with("cover."))
                {
                    let backup = parent.join(format!(
                        ".backup-{}-{}",
                        output_id,
                        path.file_name().unwrap().to_string_lossy()
                    ));
                    fs::rename(&path, &backup).map_err(|source| storage_error(&path, source))?;
                    backups.push((path, backup));
                }
            }
            if let Some(metadata) = metadata_path.as_ref()
                && metadata.exists()
            {
                let backup = parent.join(format!(".backup-{output_id}-metadata.json"));
                fs::rename(metadata, &backup).map_err(|source| storage_error(metadata, source))?;
                backups.push((metadata.clone(), backup));
            }
        }

        fs::rename(&staged.path, &target).map_err(|source| storage_error(&target, source))?;
        let managed = ManagedImage {
            path: target,
            extension: staged.extension,
            is_project_cover: save_as_cover,
        };
        if let Some(metadata) = metadata_path.as_ref() {
            write_json_atomic(metadata, &managed)?;
        }
        Ok(PendingImageCommit {
            managed,
            metadata_path,
            backups,
        })
    }

    pub fn finalize_commit(
        &self,
        pending: PendingImageCommit,
    ) -> Result<ManagedImage, MediaAssetError> {
        for (_, backup) in &pending.backups {
            if backup.exists() {
                fs::remove_file(backup).map_err(|source| storage_error(backup, source))?;
            }
        }
        Ok(pending.managed)
    }

    pub fn rollback_commit(&self, pending: PendingImageCommit) -> Result<(), MediaAssetError> {
        if pending.managed.path.exists() {
            fs::remove_file(&pending.managed.path)
                .map_err(|source| storage_error(&pending.managed.path, source))?;
        }
        if let Some(metadata) = pending.metadata_path.as_ref()
            && metadata.exists()
        {
            fs::remove_file(metadata).map_err(|source| storage_error(metadata, source))?;
        }
        for (original, backup) in pending.backups.into_iter().rev() {
            fs::rename(&backup, &original).map_err(|source| storage_error(&original, source))?;
        }
        Ok(())
    }

    pub fn record_orphans(
        &self,
        project_id: &str,
        paths: &[PathBuf],
    ) -> Result<(), MediaAssetError> {
        validate_project_id(project_id)?;
        let root = self.project_root(project_id);
        for path in paths {
            ensure_contained(&root, path)?;
        }
        fs::create_dir_all(&root).map_err(|source| storage_error(&root, source))?;
        let ledger = self.orphan_ledger(project_id);
        let mut recorded = read_orphans(&ledger)?;
        for path in paths {
            if !recorded.contains(path) {
                recorded.push(path.clone());
            }
        }
        write_json_atomic(&ledger, &recorded)
    }

    pub fn recover_project(&self, project_id: &str) -> Result<RecoveryReport, MediaAssetError> {
        validate_project_id(project_id)?;
        let root = self.project_root(project_id);
        let ledger = self.orphan_ledger(project_id);
        let paths = read_orphans(&ledger)?;
        let mut report = RecoveryReport::default();
        for path in paths {
            if let Err(error) = ensure_contained(&root, &path) {
                report.unresolved.push(CleanupFailure {
                    path,
                    message: error.to_string(),
                });
                continue;
            }
            if !path.exists() {
                report.removed.push(path);
                continue;
            }
            let result = if path.is_dir() {
                fs::remove_dir_all(&path)
            } else {
                fs::remove_file(&path)
            };
            match result {
                Ok(()) => report.removed.push(path),
                Err(error) => report.unresolved.push(CleanupFailure {
                    path,
                    message: error.to_string(),
                }),
            }
        }
        if report.unresolved.is_empty() {
            if ledger.exists() {
                fs::remove_file(&ledger).map_err(|source| storage_error(&ledger, source))?;
            }
        } else {
            let remaining: Vec<_> = report
                .unresolved
                .iter()
                .map(|failure| failure.path.clone())
                .collect();
            write_json_atomic(&ledger, &remaining)?;
        }
        Ok(report)
    }

    pub fn recover_all(&self) -> Result<Vec<RecoveryReport>, MediaAssetError> {
        let projects = self.data_dir.join("media/projects");
        if !projects.exists() {
            return Ok(Vec::new());
        }
        let mut reports = Vec::new();
        for entry in fs::read_dir(&projects).map_err(|source| storage_error(&projects, source))? {
            let entry = entry.map_err(|source| storage_error(&projects, source))?;
            if !entry
                .file_type()
                .map_err(|source| storage_error(&entry.path(), source))?
                .is_dir()
            {
                continue;
            }
            let project_id = entry.file_name().to_string_lossy().into_owned();
            let mut report = self.recover_project(&project_id)?;
            let mut partials = Vec::new();
            collect_partial_files(&entry.path(), &mut partials)?;
            for path in partials {
                match fs::remove_file(&path) {
                    Ok(()) => report.removed.push(path),
                    Err(error) => report.unresolved.push(CleanupFailure {
                        path,
                        message: error.to_string(),
                    }),
                }
            }
            if !report.unresolved.is_empty() {
                let paths: Vec<_> = report
                    .unresolved
                    .iter()
                    .map(|failure| failure.path.clone())
                    .collect();
                self.record_orphans(&project_id, &paths)?;
            }
            reports.push(report);
        }
        Ok(reports)
    }

    fn project_root(&self, project_id: &str) -> PathBuf {
        self.data_dir
            .join("media")
            .join("projects")
            .join(project_id)
    }

    fn orphan_ledger(&self, project_id: &str) -> PathBuf {
        self.project_root(project_id).join("orphan-cleanup.json")
    }
}

fn collect_partial_files(root: &Path, output: &mut Vec<PathBuf>) -> Result<(), MediaAssetError> {
    for entry in fs::read_dir(root).map_err(|source| storage_error(root, source))? {
        let entry = entry.map_err(|source| storage_error(root, source))?;
        let path = entry.path();
        if entry
            .file_type()
            .map_err(|source| storage_error(&path, source))?
            .is_dir()
        {
            collect_partial_files(&path, output)?;
        } else if path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| name.contains(".partial-"))
        {
            output.push(path);
        }
    }
    Ok(())
}

fn read_orphans(path: &Path) -> Result<Vec<PathBuf>, MediaAssetError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = fs::read(path).map_err(|source| storage_error(path, source))?;
    serde_json::from_slice(&bytes)
        .map_err(|source| storage_error(path, io::Error::new(io::ErrorKind::InvalidData, source)))
}

fn ensure_contained(root: &Path, path: &Path) -> Result<(), MediaAssetError> {
    let has_parent = path
        .components()
        .any(|component| component == Component::ParentDir);
    if has_parent || !path.starts_with(root) {
        return Err(MediaAssetError::PathOutsideProject {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), MediaAssetError> {
    let temporary = path.with_extension(format!("partial-{}", Uuid::new_v4()));
    let bytes = serde_json::to_vec_pretty(value).map_err(|source| {
        storage_error(path, io::Error::new(io::ErrorKind::InvalidData, source))
    })?;
    fs::write(&temporary, bytes).map_err(|source| storage_error(&temporary, source))?;
    fs::rename(&temporary, path).map_err(|source| storage_error(path, source))
}

fn storage_error(path: &Path, source: io::Error) -> MediaAssetError {
    MediaAssetError::Storage {
        path: path.to_path_buf(),
        source,
    }
}

fn validate_project_id(project_id: &str) -> Result<(), MediaAssetError> {
    if project_id.is_empty()
        || project_id == "."
        || project_id == ".."
        || project_id.contains(['/', '\\'])
    {
        return Err(MediaAssetError::InvalidProjectId {
            project_id: project_id.to_string(),
        });
    }
    Ok(())
}

fn normalized_extension(path: &Path) -> Result<String, MediaAssetError> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "png" | "jpg" | "jpeg" | "webp" => Ok(extension),
        _ => Err(MediaAssetError::UnsupportedImage {
            path: path.to_path_buf(),
        }),
    }
}

fn is_animated_webp(path: &Path) -> Result<bool, MediaAssetError> {
    let bytes = fs::read(path).map_err(|source| MediaAssetError::Storage {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(bytes
        .windows(4)
        .any(|window| window == b"ANIM" || window == b"ANMF"))
}
