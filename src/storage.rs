use crate::models::Job;
use anyhow::{Context, Result};
use directories::UserDirs;
use std::fs;
use std::path::PathBuf;

/// Helper to determine where to store the file safely
/// Mac/Linux: ~/Documents/career-cli/jobs.json
fn get_db_path() -> Result<PathBuf> {
    let user_dirs = UserDirs::new()
        .context("Could not determine home directory")?;
    let documents_dir = user_dirs
        .document_dir()
        .context("Could not determine Documents directory")?;
    let data_dir = documents_dir.join("career-cli");

    // Create the directory if it doesn't exist yet
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)
            .context("Failed to create data directory")?;
    }

    Ok(data_dir.join("jobs.json"))
}

pub fn load_jobs() -> Result<Vec<Job>> {
    let db_path = get_db_path()?;

    if !db_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(db_path)
        .context("Failed to read jobs.json")?;
    
    let jobs: Vec<Job> = serde_json::from_str(&content)
        .context("Failed to parse JSON")?;

    Ok(jobs)
}

pub fn save_jobs(jobs: &[Job]) -> Result<()> {
    let db_path = get_db_path()?;

    let json = serde_json::to_string_pretty(jobs)
        .context("Failed to serialize jobs")?;
    
    fs::write(db_path, json)
        .context("Failed to write to jobs.json")?;

    Ok(())
}
