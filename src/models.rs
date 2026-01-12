use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Status {
    Applied,
    Interviewing,
    Offer,
    Rejected,
    Ghosted,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Job {
    pub id: usize,
    pub company: String,
    pub role: String,
    #[serde(default)]
    pub post_link: String,
    pub status: Status,
    pub notes: String,
    pub date_applied: DateTime<Utc>,
}

impl Status {
    pub fn next(&self) -> Self {
        match self {
            Status::Applied => Status::Interviewing,
            Status::Interviewing => Status::Offer,
            Status::Offer => Status::Rejected, // Or maybe stay at Offer?
            Status::Rejected => Status::Ghosted,
            Status::Ghosted => Status::Applied,
        }
    }
}

impl Job {
    pub fn new(id: usize, company: String, role: String, post_link: String) -> Self {
        Self {
            id,
            company,
            role,
            post_link,
            status: Status::Applied,
            notes: String::new(),
            date_applied: Utc::now(),
        }
    }

    pub fn cycle_status(&mut self) {
        self.status = self.status.next();
    }
}
