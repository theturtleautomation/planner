//! # Import Store — Project-Owned Import Job Persistence
//!
//! Tracks queued import requests and their source bindings separately from the
//! canonical `Project` record.

use async_trait::async_trait;
use chrono::Utc;
use parking_lot::RwLock;
use planner_schemas::artifacts::blueprint::{BlueprintNode, NodeScope, ProjectScope, ScopeClass};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImportProvider {
    #[serde(rename = "github")]
    GitHub,
    #[serde(rename = "local")]
    Local,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportStatus {
    Queued,
    Cloning,
    Analyzing,
    ReviewPending,
    Applied,
    Failed,
}

#[derive(Debug, Clone)]
pub struct AcquiredImportSource {
    pub default_branch: String,
    pub head_revision: String,
}

#[derive(Debug, Clone, Default)]
pub struct LocalImportSourceMetadata {
    pub default_branch: Option<String>,
    pub head_revision: Option<String>,
}

#[async_trait]
pub trait ImportAcquirer: Send + Sync {
    async fn acquire_github(
        &self,
        canonical_ref: &str,
        checkout_path: &Path,
    ) -> Result<AcquiredImportSource, String>;
}

pub struct GitCliImportAcquirer;

pub fn default_import_acquirer() -> std::sync::Arc<dyn ImportAcquirer> {
    std::sync::Arc::new(GitCliImportAcquirer)
}

#[derive(Debug, Clone)]
pub struct ImportAnalysisRequest {
    pub project_id: Uuid,
    pub project_name: String,
    pub provider: ImportProvider,
    pub canonical_ref: String,
    pub local_root: PathBuf,
    pub default_branch: Option<String>,
    pub head_revision: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AnalyzedImportDraft {
    pub analysis_summary: String,
    pub discovered_nodes: Vec<BlueprintNode>,
}

#[async_trait]
pub trait ImportAnalyzer: Send + Sync {
    async fn analyze(&self, request: ImportAnalysisRequest) -> Result<AnalyzedImportDraft, String>;
}

pub struct FilesystemImportAnalyzer;

pub fn default_import_analyzer() -> std::sync::Arc<dyn ImportAnalyzer> {
    std::sync::Arc::new(FilesystemImportAnalyzer)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSourceBinding {
    pub project_id: Uuid,
    pub provider: ImportProvider,
    pub canonical_ref: String,
    #[serde(default)]
    pub default_branch: Option<String>,
    #[serde(default)]
    pub head_revision: Option<String>,
    #[serde(default)]
    pub local_root: Option<String>,
    pub managed_checkout: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectImportJob {
    pub id: Uuid,
    pub project_id: Uuid,
    pub provider: ImportProvider,
    pub requested_ref: String,
    pub status: ImportStatus,
    #[serde(default)]
    pub restored_from_job_id: Option<Uuid>,
    #[serde(default)]
    pub seed_session_id: Option<Uuid>,
    #[serde(default)]
    pub analysis_summary: Option<String>,
    #[serde(default)]
    pub progress_message: Option<String>,
    #[serde(default)]
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportDraftSourceMetadata {
    pub provider: ImportProvider,
    pub canonical_ref: String,
    pub local_root: String,
    #[serde(default)]
    pub default_branch: Option<String>,
    #[serde(default)]
    pub head_revision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectImportDraft {
    pub job_id: Uuid,
    pub project_id: Uuid,
    pub analysis_summary: String,
    pub source_metadata: ImportDraftSourceMetadata,
    #[serde(default)]
    pub discovered_nodes: Vec<BlueprintNode>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectImportReviewSelection {
    pub job_id: Uuid,
    pub project_id: Uuid,
    #[serde(default)]
    pub excluded_node_ids: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct HistoricalImportDraft {
    pub job: ProjectImportJob,
    pub draft: Option<ProjectImportDraft>,
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

pub struct ProjectImportStore {
    jobs: RwLock<HashMap<Uuid, ProjectImportJob>>,
    bindings: RwLock<HashMap<Uuid, ProjectSourceBinding>>,
    drafts: RwLock<HashMap<Uuid, ProjectImportDraft>>,
    selections: RwLock<HashMap<Uuid, ProjectImportReviewSelection>>,
    jobs_dir: PathBuf,
    bindings_dir: PathBuf,
    drafts_dir: PathBuf,
    selections_dir: PathBuf,
    checkouts_dir: PathBuf,
    persistent: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectImportCleanupReport {
    pub jobs_deleted: usize,
    pub drafts_deleted: usize,
    pub managed_roots_deleted: usize,
}

impl ProjectImportStore {
    pub fn new() -> Self {
        let imports_dir = std::env::temp_dir().join("planner-imports-in-memory");
        let jobs_dir = imports_dir.join("jobs");
        let bindings_dir = imports_dir.join("bindings");
        let drafts_dir = imports_dir.join("drafts");
        let selections_dir = imports_dir.join("selections");
        let checkouts_dir = imports_dir.join("checkouts");
        Self {
            jobs: RwLock::new(HashMap::new()),
            bindings: RwLock::new(HashMap::new()),
            drafts: RwLock::new(HashMap::new()),
            selections: RwLock::new(HashMap::new()),
            jobs_dir,
            bindings_dir,
            drafts_dir,
            selections_dir,
            checkouts_dir,
            persistent: false,
        }
    }

    pub fn open(data_dir: &Path) -> std::io::Result<Self> {
        let imports_dir = data_dir.join("imports");
        let jobs_dir = imports_dir.join("jobs");
        let bindings_dir = imports_dir.join("bindings");
        let drafts_dir = imports_dir.join("drafts");
        let selections_dir = imports_dir.join("selections");
        let checkouts_dir = imports_dir.join("checkouts");
        std::fs::create_dir_all(&jobs_dir)?;
        std::fs::create_dir_all(&bindings_dir)?;
        std::fs::create_dir_all(&drafts_dir)?;
        std::fs::create_dir_all(&selections_dir)?;
        std::fs::create_dir_all(&checkouts_dir)?;

        let jobs = load_records::<ProjectImportJob, _>(&jobs_dir, |job| job.id)?;
        let bindings =
            load_records::<ProjectSourceBinding, _>(&bindings_dir, |binding| binding.project_id)?;
        let drafts = load_records::<ProjectImportDraft, _>(&drafts_dir, |draft| draft.job_id)?;
        let selections =
            load_records::<ProjectImportReviewSelection, _>(&selections_dir, |selection| {
                selection.job_id
            })?;

        Ok(Self {
            jobs: RwLock::new(jobs),
            bindings: RwLock::new(bindings),
            drafts: RwLock::new(drafts),
            selections: RwLock::new(selections),
            jobs_dir,
            bindings_dir,
            drafts_dir,
            selections_dir,
            checkouts_dir,
            persistent: true,
        })
    }

    pub fn count_jobs(&self) -> usize {
        self.jobs.read().len()
    }

    pub fn is_persistent(&self) -> bool {
        self.persistent
    }

    pub fn create(
        &self,
        project_id: Uuid,
        provider: ImportProvider,
        requested_ref: String,
        canonical_ref: String,
        managed_checkout: bool,
    ) -> std::io::Result<(ProjectImportJob, ProjectSourceBinding)> {
        let now = now_rfc3339();
        let job = ProjectImportJob {
            id: Uuid::new_v4(),
            project_id,
            provider,
            requested_ref,
            status: ImportStatus::Queued,
            restored_from_job_id: None,
            seed_session_id: None,
            analysis_summary: None,
            progress_message: Some("Import request queued".into()),
            error_message: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let binding = ProjectSourceBinding {
            project_id,
            provider,
            canonical_ref,
            default_branch: None,
            head_revision: None,
            local_root: None,
            managed_checkout,
            created_at: now.clone(),
            updated_at: now,
        };

        self.persist_job(&job)?;
        self.persist_binding(&binding)?;

        self.jobs.write().insert(job.id, job.clone());
        self.bindings.write().insert(project_id, binding.clone());

        Ok((job, binding))
    }

    pub fn create_reimport_job(
        &self,
        project_id: Uuid,
    ) -> std::io::Result<(ProjectImportJob, ProjectSourceBinding)> {
        let binding = self.get_binding(project_id).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("import binding not found for project: {project_id}"),
            )
        })?;
        let now = now_rfc3339();
        let job = ProjectImportJob {
            id: Uuid::new_v4(),
            project_id,
            provider: binding.provider,
            requested_ref: binding.canonical_ref.clone(),
            status: ImportStatus::Queued,
            restored_from_job_id: None,
            seed_session_id: None,
            analysis_summary: None,
            progress_message: Some("Re-import request queued".into()),
            error_message: None,
            created_at: now.clone(),
            updated_at: now,
        };
        self.persist_job(&job)?;
        self.jobs.write().insert(job.id, job.clone());
        self.reset_review_selection(job.id, project_id)?;
        Ok((job, binding))
    }

    pub fn create_restore_job(
        &self,
        project_id: Uuid,
        restored_from_job_id: Uuid,
    ) -> std::io::Result<(ProjectImportJob, ProjectSourceBinding)> {
        let binding = self.get_binding(project_id).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("import binding not found for project: {project_id}"),
            )
        })?;
        let now = now_rfc3339();
        let job = ProjectImportJob {
            id: Uuid::new_v4(),
            project_id,
            provider: binding.provider,
            requested_ref: binding.canonical_ref.clone(),
            status: ImportStatus::Queued,
            restored_from_job_id: Some(restored_from_job_id),
            seed_session_id: None,
            analysis_summary: None,
            progress_message: Some(format!(
                "Historical restore queued from import {}",
                restored_from_job_id
            )),
            error_message: None,
            created_at: now.clone(),
            updated_at: now,
        };
        self.persist_job(&job)?;
        self.jobs.write().insert(job.id, job.clone());
        Ok((job, binding))
    }

    pub fn create_restore_review_job(
        &self,
        project_id: Uuid,
        restored_from_job_id: Uuid,
        analysis_summary: Option<String>,
        seed_session_id: Option<Uuid>,
    ) -> std::io::Result<(ProjectImportJob, ProjectSourceBinding)> {
        let binding = self.get_binding(project_id).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("import binding not found for project: {project_id}"),
            )
        })?;
        let now = now_rfc3339();
        let job = ProjectImportJob {
            id: Uuid::new_v4(),
            project_id,
            provider: binding.provider,
            requested_ref: binding.canonical_ref.clone(),
            status: ImportStatus::ReviewPending,
            restored_from_job_id: Some(restored_from_job_id),
            seed_session_id,
            analysis_summary,
            progress_message: Some(format!(
                "Historical review draft restored from import {}. Review and apply when ready.",
                restored_from_job_id
            )),
            error_message: None,
            created_at: now.clone(),
            updated_at: now,
        };
        self.persist_job(&job)?;
        self.jobs.write().insert(job.id, job.clone());
        Ok((job, binding))
    }

    pub fn get_job(&self, id: Uuid) -> Option<ProjectImportJob> {
        self.jobs.read().get(&id).cloned()
    }

    pub fn get_binding(&self, project_id: Uuid) -> Option<ProjectSourceBinding> {
        self.bindings.read().get(&project_id).cloned()
    }

    pub fn find_binding_by_source(
        &self,
        provider: ImportProvider,
        canonical_ref: &str,
    ) -> Option<ProjectSourceBinding> {
        self.bindings
            .read()
            .values()
            .find(|binding| binding.provider == provider && binding.canonical_ref == canonical_ref)
            .cloned()
    }

    pub fn get_draft(&self, job_id: Uuid) -> Option<ProjectImportDraft> {
        self.drafts.read().get(&job_id).cloned()
    }

    pub fn get_review_selection(&self, job_id: Uuid) -> Option<ProjectImportReviewSelection> {
        self.selections.read().get(&job_id).cloned()
    }

    pub fn latest_job_for_project(&self, project_id: Uuid) -> Option<ProjectImportJob> {
        self.jobs
            .read()
            .values()
            .filter(|job| job.project_id == project_id)
            .cloned()
            .max_by(|left, right| left.updated_at.cmp(&right.updated_at))
    }

    pub fn latest_review_job_for_project(&self, project_id: Uuid) -> Option<ProjectImportJob> {
        self.jobs
            .read()
            .values()
            .filter(|job| {
                job.project_id == project_id
                    && matches!(
                        job.status,
                        ImportStatus::ReviewPending | ImportStatus::Applied
                    )
            })
            .cloned()
            .max_by(|left, right| left.updated_at.cmp(&right.updated_at))
    }

    pub fn latest_applied_job_for_project(&self, project_id: Uuid) -> Option<ProjectImportJob> {
        self.jobs
            .read()
            .values()
            .filter(|job| {
                job.project_id == project_id && matches!(job.status, ImportStatus::Applied)
            })
            .cloned()
            .max_by(|left, right| left.updated_at.cmp(&right.updated_at))
    }

    pub fn history_for_project(&self, project_id: Uuid) -> Vec<HistoricalImportDraft> {
        let jobs = self.jobs.read();
        let drafts = self.drafts.read();
        let mut history = jobs
            .values()
            .filter(|job| job.project_id == project_id)
            .cloned()
            .map(|job| HistoricalImportDraft {
                draft: drafts.get(&job.id).cloned(),
                job,
            })
            .collect::<Vec<_>>();
        history.sort_by(|left, right| {
            right
                .job
                .updated_at
                .cmp(&left.job.updated_at)
                .then_with(|| right.job.created_at.cmp(&left.job.created_at))
                .then_with(|| right.job.id.cmp(&left.job.id))
        });
        history
    }

    pub fn managed_checkout_path(&self, project_id: Uuid, provider: ImportProvider) -> PathBuf {
        let provider_dir = match provider {
            ImportProvider::GitHub => "github",
            ImportProvider::Local => "local",
        };
        self.checkouts_dir
            .join(provider_dir)
            .join(project_id.to_string())
    }

    pub fn mark_job_cloning(
        &self,
        job_id: Uuid,
        progress_message: impl Into<String>,
    ) -> std::io::Result<ProjectImportJob> {
        self.update_job(job_id, |job| {
            job.status = ImportStatus::Cloning;
            job.progress_message = Some(progress_message.into());
            job.error_message = None;
        })
    }

    pub fn mark_job_analyzing(
        &self,
        job_id: Uuid,
        progress_message: impl Into<String>,
    ) -> std::io::Result<ProjectImportJob> {
        self.update_job(job_id, |job| {
            job.status = ImportStatus::Analyzing;
            job.progress_message = Some(progress_message.into());
            job.error_message = None;
        })
    }

    pub fn mark_job_review_pending(
        &self,
        job_id: Uuid,
        progress_message: impl Into<String>,
        analysis_summary: String,
        seed_session_id: Uuid,
    ) -> std::io::Result<ProjectImportJob> {
        let job = self.update_job(job_id, |job| {
            job.status = ImportStatus::ReviewPending;
            job.seed_session_id = Some(seed_session_id);
            job.analysis_summary = Some(analysis_summary);
            job.progress_message = Some(progress_message.into());
            job.error_message = None;
        })?;
        self.reset_review_selection(job.id, job.project_id)?;
        Ok(job)
    }

    pub fn mark_job_applied(
        &self,
        job_id: Uuid,
        progress_message: impl Into<String>,
        restored_from_job_id: Option<Uuid>,
    ) -> std::io::Result<ProjectImportJob> {
        self.update_job(job_id, |job| {
            job.status = ImportStatus::Applied;
            job.restored_from_job_id = restored_from_job_id;
            job.progress_message = Some(progress_message.into());
            job.error_message = None;
        })
    }

    pub fn mark_job_failed(
        &self,
        job_id: Uuid,
        error_message: impl Into<String>,
    ) -> std::io::Result<ProjectImportJob> {
        self.update_job(job_id, |job| {
            job.status = ImportStatus::Failed;
            job.progress_message = None;
            job.error_message = Some(error_message.into());
        })
    }

    pub fn update_binding_source_metadata(
        &self,
        project_id: Uuid,
        default_branch: Option<String>,
        head_revision: Option<String>,
        local_root: String,
    ) -> std::io::Result<ProjectSourceBinding> {
        self.update_binding(project_id, |binding| {
            binding.default_branch = default_branch;
            binding.head_revision = head_revision;
            binding.local_root = Some(local_root);
        })
    }

    pub fn save_draft(&self, draft: ProjectImportDraft) -> std::io::Result<ProjectImportDraft> {
        let mut draft = draft;
        draft.updated_at = now_rfc3339();
        self.persist_draft(&draft)?;
        self.drafts.write().insert(draft.job_id, draft.clone());
        Ok(draft)
    }

    pub fn reset_review_selection(
        &self,
        job_id: Uuid,
        project_id: Uuid,
    ) -> std::io::Result<ProjectImportReviewSelection> {
        let now = now_rfc3339();
        let selection = ProjectImportReviewSelection {
            job_id,
            project_id,
            excluded_node_ids: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        };
        self.persist_selection(&selection)?;
        self.selections.write().insert(job_id, selection.clone());
        Ok(selection)
    }

    pub fn set_review_node_included(
        &self,
        job_id: Uuid,
        project_id: Uuid,
        node_id: &str,
        included: bool,
    ) -> std::io::Result<ProjectImportReviewSelection> {
        if self.get_review_selection(job_id).is_none() {
            self.reset_review_selection(job_id, project_id)?;
        }

        self.update_selection(job_id, |selection| {
            if included {
                selection
                    .excluded_node_ids
                    .retain(|current| current != node_id);
            } else if !selection
                .excluded_node_ids
                .iter()
                .any(|current| current == node_id)
            {
                selection.excluded_node_ids.push(node_id.to_string());
            }
            selection.excluded_node_ids.sort();
            selection.excluded_node_ids.dedup();
        })
    }

    pub fn purge_project(&self, project_id: Uuid) -> std::io::Result<ProjectImportCleanupReport> {
        let binding = self.get_binding(project_id);
        let job_ids = self
            .jobs
            .read()
            .values()
            .filter(|job| job.project_id == project_id)
            .map(|job| job.id)
            .collect::<Vec<_>>();
        let drafts_deleted = self
            .drafts
            .read()
            .values()
            .filter(|draft| draft.project_id == project_id)
            .count();

        for job_id in &job_ids {
            let path = self.jobs_dir.join(format!("{}.msgpack", job_id));
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
            let draft_path = self.drafts_dir.join(format!("{}.msgpack", job_id));
            if draft_path.exists() {
                std::fs::remove_file(&draft_path)?;
            }
            let selection_path = self.selections_dir.join(format!("{}.msgpack", job_id));
            if selection_path.exists() {
                std::fs::remove_file(&selection_path)?;
            }
        }

        let binding_path = self.bindings_dir.join(format!("{}.msgpack", project_id));
        if binding_path.exists() {
            std::fs::remove_file(&binding_path)?;
        }

        let mut managed_roots_deleted = 0usize;
        if let Some(binding) = binding.as_ref().filter(|binding| binding.managed_checkout) {
            let managed_root = self.managed_checkout_path(project_id, binding.provider);
            if managed_root.exists() {
                std::fs::remove_dir_all(&managed_root)?;
                managed_roots_deleted += 1;
            }
        }

        self.jobs
            .write()
            .retain(|_, job| job.project_id != project_id);
        self.drafts
            .write()
            .retain(|_, draft| draft.project_id != project_id);
        self.selections
            .write()
            .retain(|_, selection| selection.project_id != project_id);
        self.bindings.write().remove(&project_id);

        Ok(ProjectImportCleanupReport {
            jobs_deleted: job_ids.len(),
            drafts_deleted,
            managed_roots_deleted,
        })
    }

    fn persist_job(&self, job: &ProjectImportJob) -> std::io::Result<()> {
        persist_record(&self.jobs_dir, &job.id.to_string(), job)?;
        Ok(())
    }

    fn persist_binding(&self, binding: &ProjectSourceBinding) -> std::io::Result<()> {
        persist_record(&self.bindings_dir, &binding.project_id.to_string(), binding)?;
        Ok(())
    }

    fn persist_draft(&self, draft: &ProjectImportDraft) -> std::io::Result<()> {
        persist_record(&self.drafts_dir, &draft.job_id.to_string(), draft)?;
        Ok(())
    }

    fn persist_selection(&self, selection: &ProjectImportReviewSelection) -> std::io::Result<()> {
        persist_record(
            &self.selections_dir,
            &selection.job_id.to_string(),
            selection,
        )?;
        Ok(())
    }

    fn update_job<F>(&self, job_id: Uuid, update: F) -> std::io::Result<ProjectImportJob>
    where
        F: FnOnce(&mut ProjectImportJob),
    {
        let updated = {
            let mut jobs = self.jobs.write();
            let job = jobs.get_mut(&job_id).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("import job not found: {job_id}"),
                )
            })?;
            update(job);
            job.updated_at = now_rfc3339();
            job.clone()
        };
        self.persist_job(&updated)?;
        Ok(updated)
    }

    fn update_binding<F>(
        &self,
        project_id: Uuid,
        update: F,
    ) -> std::io::Result<ProjectSourceBinding>
    where
        F: FnOnce(&mut ProjectSourceBinding),
    {
        let updated = {
            let mut bindings = self.bindings.write();
            let binding = bindings.get_mut(&project_id).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("import binding not found for project: {project_id}"),
                )
            })?;
            update(binding);
            binding.updated_at = now_rfc3339();
            binding.clone()
        };
        self.persist_binding(&updated)?;
        Ok(updated)
    }

    fn update_selection<F>(
        &self,
        job_id: Uuid,
        update: F,
    ) -> std::io::Result<ProjectImportReviewSelection>
    where
        F: FnOnce(&mut ProjectImportReviewSelection),
    {
        let updated = {
            let mut selections = self.selections.write();
            let selection = selections.get_mut(&job_id).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("import review selection not found: {job_id}"),
                )
            })?;
            update(selection);
            selection.updated_at = now_rfc3339();
            selection.clone()
        };
        self.persist_selection(&updated)?;
        Ok(updated)
    }
}

fn load_records<T, F>(dir: &Path, key_fn: F) -> std::io::Result<HashMap<Uuid, T>>
where
    T: for<'de> Deserialize<'de>,
    F: Fn(&T) -> Uuid,
{
    let mut out = HashMap::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if !file_name.ends_with(".msgpack") || file_name.ends_with(".tmp") {
            continue;
        }

        let bytes = std::fs::read(&path)?;
        let record: T = rmp_serde::from_slice(&bytes)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string()))?;
        out.insert(key_fn(&record), record);
    }
    Ok(out)
}

fn persist_record<T: Serialize>(dir: &Path, id: &str, value: &T) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let bytes = rmp_serde::to_vec_named(value)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string()))?;
    let path = dir.join(format!("{id}.msgpack"));
    let tmp_path = dir.join(format!("{id}.msgpack.tmp"));
    std::fs::write(&tmp_path, bytes)?;
    std::fs::rename(tmp_path, path)?;
    Ok(())
}

#[async_trait]
impl ImportAcquirer for GitCliImportAcquirer {
    async fn acquire_github(
        &self,
        canonical_ref: &str,
        checkout_path: &Path,
    ) -> Result<AcquiredImportSource, String> {
        if checkout_path.exists() {
            std::fs::remove_dir_all(checkout_path).map_err(|err| {
                format!(
                    "failed to clear existing checkout {}: {}",
                    checkout_path.display(),
                    err
                )
            })?;
        }
        if let Some(parent) = checkout_path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                format!(
                    "failed to create checkout parent {}: {}",
                    parent.display(),
                    err
                )
            })?;
        }

        let default_branch = resolve_remote_default_branch(canonical_ref).await?;
        run_git(&[
            "clone",
            "--depth",
            "1",
            "--branch",
            &default_branch,
            canonical_ref,
            &checkout_path.to_string_lossy(),
        ])
        .await?;
        let head_revision = run_git(&["-C", &checkout_path.to_string_lossy(), "rev-parse", "HEAD"])
            .await?
            .trim()
            .to_string();

        Ok(AcquiredImportSource {
            default_branch,
            head_revision,
        })
    }
}

pub async fn inspect_local_import_source(
    local_root: &Path,
) -> Result<LocalImportSourceMetadata, String> {
    let metadata = run_git(&[
        "-C",
        &local_root.to_string_lossy(),
        "rev-parse",
        "--is-inside-work-tree",
    ])
    .await;

    if metadata.is_err() {
        return Ok(LocalImportSourceMetadata::default());
    }

    let head_revision = run_git(&["-C", &local_root.to_string_lossy(), "rev-parse", "HEAD"])
        .await
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let default_branch = run_git(&[
        "-C",
        &local_root.to_string_lossy(),
        "branch",
        "--show-current",
    ])
    .await
    .ok()
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty());

    Ok(LocalImportSourceMetadata {
        default_branch,
        head_revision,
    })
}

#[async_trait]
impl ImportAnalyzer for FilesystemImportAnalyzer {
    async fn analyze(&self, request: ImportAnalysisRequest) -> Result<AnalyzedImportDraft, String> {
        if !request.local_root.exists() || !request.local_root.is_dir() {
            return Err(format!(
                "prepared checkout is unavailable at {}",
                request.local_root.display()
            ));
        }

        let scratch_blueprints = planner_core::blueprint::BlueprintStore::new();
        let mut draft_nodes = Vec::new();
        draft_nodes.extend(
            planner_core::discovery::scan_directory_structure(
                &request.local_root,
                &scratch_blueprints,
            )
            .proposals
            .into_iter()
            .map(|proposal| proposal.node),
        );
        draft_nodes.extend(
            planner_core::discovery::scan_cargo_toml(&request.local_root, &scratch_blueprints)
                .proposals
                .into_iter()
                .map(|proposal| proposal.node),
        );

        let project_scope = NodeScope {
            scope_class: ScopeClass::Project,
            project: Some(ProjectScope {
                project_id: request.project_id.to_string(),
                project_name: Some(request.project_name.clone()),
            }),
            secondary: Default::default(),
            is_shared: false,
            shared: None,
            lifecycle: Default::default(),
            override_scope: None,
            scope_review: None,
        };
        for node in &mut draft_nodes {
            apply_project_scope(node, &project_scope);
        }

        let component_names = draft_nodes
            .iter()
            .filter_map(|node| match node {
                BlueprintNode::Component(component) => Some(component.name.clone()),
                _ => None,
            })
            .take(4)
            .collect::<Vec<_>>();
        let technology_names = draft_nodes
            .iter()
            .filter_map(|node| match node {
                BlueprintNode::Technology(technology) => Some(technology.name.clone()),
                _ => None,
            })
            .take(4)
            .collect::<Vec<_>>();

        let readme_summary = summarize_top_level_readme(&request.local_root);
        let cargo_manifest_count = collect_paths_named(&request.local_root, "Cargo.toml").len();
        let branch_note = request
            .default_branch
            .as_deref()
            .map(|branch| match request.head_revision.as_deref() {
                Some(revision) if revision.len() >= 8 => {
                    format!("Checked out `{branch}` at `{}`.", &revision[..8])
                }
                Some(revision) => format!("Checked out `{branch}` at `{revision}`."),
                None => format!("Checked out `{branch}`."),
            })
            .unwrap_or_else(|| "Checkout metadata is available.".into());

        let mut summary_lines = vec![
            format!(
                "Imported draft for {} from {}.",
                request.project_name, request.canonical_ref
            ),
            branch_note,
        ];
        if let Some(readme_summary) = readme_summary {
            summary_lines.push(format!("Repository brief: {}", readme_summary));
        }
        if !component_names.is_empty() {
            summary_lines.push(format!(
                "Discovered draft components: {}.",
                component_names.join(", ")
            ));
        }
        if !technology_names.is_empty() {
            summary_lines.push(format!(
                "Detected Rust technology dependencies: {}.",
                technology_names.join(", ")
            ));
        }
        if cargo_manifest_count > 0 {
            summary_lines.push(format!(
                "Cargo/workspace manifests found: {}.",
                cargo_manifest_count
            ));
        }
        if draft_nodes.is_empty() {
            summary_lines.push(
                "No structured components or technologies were inferred yet; continue review from the repository brief."
                    .into(),
            );
        }

        Ok(AnalyzedImportDraft {
            analysis_summary: summary_lines.join(" "),
            discovered_nodes: draft_nodes,
        })
    }
}

async fn resolve_remote_default_branch(canonical_ref: &str) -> Result<String, String> {
    let output = run_git(&["ls-remote", "--symref", canonical_ref, "HEAD"]).await?;
    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("ref: refs/heads/") {
            if let Some((branch, target)) = rest.split_once('\t') {
                if target == "HEAD" {
                    return Ok(branch.to_string());
                }
            }
        }
    }
    Err(format!(
        "could not resolve remote default branch for {}",
        canonical_ref
    ))
}

async fn run_git(args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .await
        .map_err(|err| format!("failed to run git {:?}: {}", args, err))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(format!("git {:?} failed: {}", args, detail));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn apply_project_scope(node: &mut BlueprintNode, scope: &NodeScope) {
    match node {
        BlueprintNode::Project(project) => project.scope = scope.clone(),
        BlueprintNode::Decision(decision) => decision.scope = scope.clone(),
        BlueprintNode::Technology(technology) => technology.scope = scope.clone(),
        BlueprintNode::Component(component) => component.scope = scope.clone(),
        BlueprintNode::Constraint(constraint) => constraint.scope = scope.clone(),
        BlueprintNode::Pattern(pattern) => pattern.scope = scope.clone(),
        BlueprintNode::QualityRequirement(requirement) => requirement.scope = scope.clone(),
    }
}

fn summarize_top_level_readme(project_root: &Path) -> Option<String> {
    let candidates = ["README.md", "README", "readme.md", "readme"];
    for candidate in candidates {
        let path = project_root.join(candidate);
        let Ok(contents) = std::fs::read_to_string(&path) else {
            continue;
        };
        let normalized = contents
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .take(6)
            .map(|line| line.trim_start_matches('#').trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if !normalized.is_empty() {
            let clipped = if normalized.len() > 280 {
                format!("{}…", &normalized[..280].trim_end())
            } else {
                normalized
            };
            return Some(clipped);
        }
    }
    None
}

fn collect_paths_named(root: &Path, file_name: &str) -> Vec<PathBuf> {
    let mut matches = Vec::new();
    visit_paths(root, &mut |path| {
        if path
            .file_name()
            .and_then(|value| value.to_str())
            .map(|value| value == file_name)
            .unwrap_or(false)
        {
            matches.push(path.to_path_buf());
        }
    });
    matches
}

fn visit_paths(root: &Path, visit: &mut impl FnMut(&Path)) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .file_name()
            .and_then(|value| value.to_str())
            .map(|value| value.starts_with('.'))
            .unwrap_or(false)
            && path.is_dir()
        {
            continue;
        }
        if path.is_dir() {
            visit_paths(&path, visit);
        } else {
            visit(&path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command as StdCommand;

    #[test]
    fn import_store_round_trips_records_from_disk() {
        let data_dir =
            std::env::temp_dir().join(format!("planner_import_store_{}", Uuid::new_v4()));
        let store = ProjectImportStore::open(&data_dir).unwrap();
        let project_id = Uuid::new_v4();

        let (job, binding) = store
            .create(
                project_id,
                ImportProvider::GitHub,
                "https://github.com/example/repo".into(),
                "https://github.com/example/repo".into(),
                true,
            )
            .unwrap();

        assert_eq!(store.count_jobs(), 1);
        assert!(store.get_job(job.id).is_some());
        assert!(store.get_binding(project_id).is_some());

        let loaded = ProjectImportStore::open(&data_dir).unwrap();
        let loaded_job = loaded.get_job(job.id).unwrap();
        let loaded_binding = loaded.get_binding(project_id).unwrap();
        assert_eq!(loaded_job.project_id, project_id);
        assert_eq!(loaded_binding.canonical_ref, binding.canonical_ref);
        assert!(loaded.is_persistent());

        let _ = std::fs::remove_dir_all(data_dir);
    }

    #[test]
    fn import_store_persists_job_and_binding_updates() {
        let data_dir =
            std::env::temp_dir().join(format!("planner_import_store_updates_{}", Uuid::new_v4()));
        let store = ProjectImportStore::open(&data_dir).unwrap();
        let project_id = Uuid::new_v4();

        let (job, _) = store
            .create(
                project_id,
                ImportProvider::GitHub,
                "https://github.com/example/repo".into(),
                "https://github.com/example/repo".into(),
                true,
            )
            .unwrap();
        store
            .mark_job_cloning(job.id, "Cloning default branch into managed storage")
            .unwrap();
        store
            .update_binding_source_metadata(
                project_id,
                Some("main".into()),
                Some("abc123".into()),
                "/tmp/checkouts/project".into(),
            )
            .unwrap();
        store
            .save_draft(ProjectImportDraft {
                job_id: job.id,
                project_id,
                analysis_summary: "Imported draft summary".into(),
                source_metadata: ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/repo".into(),
                    local_root: "/tmp/checkouts/project".into(),
                    default_branch: Some("main".into()),
                    head_revision: Some("abc123".into()),
                },
                discovered_nodes: Vec::new(),
                created_at: now_rfc3339(),
                updated_at: now_rfc3339(),
            })
            .unwrap();
        store
            .mark_job_review_pending(
                job.id,
                "Import draft ready",
                "Imported draft summary".into(),
                Uuid::new_v4(),
            )
            .unwrap();

        let loaded = ProjectImportStore::open(&data_dir).unwrap();
        let loaded_job = loaded.get_job(job.id).unwrap();
        let loaded_binding = loaded.get_binding(project_id).unwrap();
        let loaded_draft = loaded.get_draft(job.id).unwrap();
        assert_eq!(loaded_job.status, ImportStatus::ReviewPending);
        assert_eq!(
            loaded_job.analysis_summary.as_deref(),
            Some("Imported draft summary")
        );
        assert!(loaded_job.seed_session_id.is_some());
        assert_eq!(loaded_binding.default_branch.as_deref(), Some("main"));
        assert_eq!(loaded_binding.head_revision.as_deref(), Some("abc123"));
        assert_eq!(
            loaded_binding.local_root.as_deref(),
            Some("/tmp/checkouts/project")
        );
        assert_eq!(loaded_draft.project_id, project_id);
        assert_eq!(loaded_draft.analysis_summary, "Imported draft summary");

        let _ = std::fs::remove_dir_all(data_dir);
    }

    #[tokio::test]
    async fn git_cli_acquirer_clones_temp_bare_repo_and_reads_metadata() {
        let root = std::env::temp_dir().join(format!("planner_git_import_{}", Uuid::new_v4()));
        let source = root.join("source");
        let remote = root.join("github-mirror").join("example").join("repo");
        let checkout = root.join("checkout");
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::create_dir_all(remote.parent().unwrap()).unwrap();

        run_git_sync(&[
            "init",
            "--initial-branch",
            "main",
            &source.to_string_lossy(),
        ]);
        run_git_sync(&[
            "-C",
            &source.to_string_lossy(),
            "config",
            "user.email",
            "planner@example.com",
        ]);
        run_git_sync(&[
            "-C",
            &source.to_string_lossy(),
            "config",
            "user.name",
            "Planner Test",
        ]);
        std::fs::write(source.join("README.md"), "# Example\n").unwrap();
        run_git_sync(&["-C", &source.to_string_lossy(), "add", "README.md"]);
        run_git_sync(&["-C", &source.to_string_lossy(), "commit", "-m", "initial"]);
        run_git_sync(&[
            "clone",
            "--bare",
            &source.to_string_lossy(),
            &remote.to_string_lossy(),
        ]);

        let acquirer = GitCliImportAcquirer;
        let acquired = acquirer
            .acquire_github(&format!("file://{}", remote.to_string_lossy()), &checkout)
            .await
            .unwrap();

        assert_eq!(acquired.default_branch, "main");
        assert!(!acquired.head_revision.trim().is_empty());
        assert!(checkout.join("README.md").exists());

        let _ = std::fs::remove_dir_all(root);
    }

    fn run_git_sync(args: &[&str]) {
        let output = StdCommand::new("git").args(args).output().unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
