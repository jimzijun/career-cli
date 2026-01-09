use crate::models::Job;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

const DB_FILE: &str = "jobs.json";

/// Loads jobs from the JSON file. Returns an empty list if file doesn't exist.
pub fn load_jobs() -> Result<Vec<Job>> {
    if !Path::new(DB_FILE).exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(DB_FILE)
        .context("Failed to read jobs.json")?;
    
    let jobs: Vec<Job> = serde_json::from_str(&content)
        .context("Failed to parse JSON")?;

    Ok(jobs)
}

/// Saves the current list of jobs to the JSON file.
pub fn save_jobs(jobs: &[Job]) -> Result<()> {
    let json = serde_json::to_string_pretty(jobs)
        .context("Failed to serialize jobs")?;
    
    fs::write(DB_FILE, json)
        .context("Failed to write to jobs.json")?;

    Ok(())
}