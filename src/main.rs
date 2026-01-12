mod models;
mod storage;

use std::io;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use ratatui::widgets::Clear; // Add this import at top of file
 // Import Status to match against it
use models::Job;
use storage::{load_jobs, save_jobs};
use ratatui::widgets::{List, ListItem, ListState}; // Updated imports
use ratatui::style::{Color, Modifier, Style};

// Track which screen/mode we are in
enum InputMode {
    Normal,
    Editing,
}

// Track which field user is currently typing
enum InputField {
    Company,
    Role,
    Link,
}

enum EditTarget {
    New,
    Existing(usize),
}

struct App {
    jobs: Vec<Job>,
    state: ListState,
    should_quit: bool,
    // --- NEW FIELDS ---
    input_mode: InputMode,
    input_field: InputField,
    input_buffer: String,      // What user is currently typing
    temp_company: String,      // Store company while typing role
    temp_role: String,         // Store role while typing link
    edit_target: EditTarget,
}

impl App {
    fn new(jobs: Vec<Job>) -> Self {
        let mut state = ListState::default();
        if !jobs.is_empty() { state.select(Some(0)); }
        
        Self {
            jobs,
            state,
            should_quit: false,
            // Initialize new fields
            input_mode: InputMode::Normal,
            input_field: InputField::Company,
            input_buffer: String::new(),
            temp_company: String::new(),
            temp_role: String::new(),
            edit_target: EditTarget::New,
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.jobs.len() - 1 {
                    0 // Wrap around to top
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.jobs.len() - 1 // Wrap around to bottom
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn submit_input(&mut self) {
        match self.input_field {
            InputField::Company => {
                // Save company, switch to Role field
                self.temp_company = self.input_buffer.clone();
                self.input_buffer.clear();
                self.input_field = InputField::Role;
            }
            InputField::Role => {
                self.temp_role = self.input_buffer.clone();
                self.input_buffer.clear();
                self.input_field = InputField::Link;
            }
            InputField::Link => {
                let post_link = self.input_buffer.trim().to_string();
                match self.edit_target {
                    EditTarget::New => {
                        let new_id = self.jobs.len() + 1;
                        let new_job = Job::new(
                            new_id,
                            self.temp_company.clone(),
                            self.temp_role.clone(),
                            post_link,
                        );
                        self.jobs.push(new_job);
                    }
                    EditTarget::Existing(index) => {
                        if let Some(job) = self.jobs.get_mut(index) {
                            job.post_link = post_link;
                        }
                    }
                }
                self.reset_input();
            }
        }
    }

    fn reset_input(&mut self) {
        self.input_buffer.clear();
        self.temp_company.clear();
        self.temp_role.clear();
        self.edit_target = EditTarget::New;
        self.input_mode = InputMode::Normal;
        self.input_field = InputField::Company;
    }

    fn start_add(&mut self) {
        self.input_mode = InputMode::Editing;
        self.input_field = InputField::Company;
        self.edit_target = EditTarget::New;
        self.input_buffer.clear();
    }

    fn start_edit_link(&mut self) {
        if let Some(i) = self.state.selected() {
            if let Some(job) = self.jobs.get(i) {
                self.input_mode = InputMode::Editing;
                self.input_field = InputField::Link;
                self.edit_target = EditTarget::Existing(i);
                self.input_buffer = job.post_link.clone();
            }
        }
    }

    fn cycle_current_status(&mut self) {
        if let Some(i) = self.state.selected() {
            if let Some(job) = self.jobs.get_mut(i) {
                job.cycle_status();
            }
        }
    }

    fn open_current_link(&self) {
        if let Some(i) = self.state.selected() {
            if let Some(job) = self.jobs.get(i) {
                if !job.post_link.trim().is_empty() {
                    let _ = open::that(&job.post_link);
                }
            }
        }
    }

    fn delete_current_job(&mut self) {
        if let Some(i) = self.state.selected() {
            if i < self.jobs.len() {
                self.jobs.remove(i);
                
                // Adjust selection if we deleted the last item
                if !self.jobs.is_empty() && i >= self.jobs.len() {
                    self.state.select(Some(self.jobs.len() - 1));
                } else if self.jobs.is_empty() {
                    self.state.select(None);
                }
            }
        }
    }
}

fn main() -> Result<()> {
    // --- 1. SETUP TERMINAL ---
    enable_raw_mode()?; // Turn off echo and line buffering
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?; // Enter a new clean screen
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // --- 2. INITIALIZE STATE ---
    let jobs = load_jobs()?;
    let mut app = App::new(jobs);

    // --- 3. RUN APP LOOP ---
    let res = run_app(&mut terminal, &mut app);

    // --- 4. CLEANUP (Must happen even if app crashes) ---
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // If the loop failed, print the error after cleanup
    if let Err(err) = res {
        println!("{:?}", err);
    } else {
        // Save on clean exit
        save_jobs(&app.jobs)?;
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(std::time::Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match app.input_mode {
                    // --- NORMAL MODE ---
                        InputMode::Normal => match key.code {
                        KeyCode::Char('q') => app.should_quit = true,
                        KeyCode::Down => app.next(),
                        KeyCode::Up => app.previous(),
                        KeyCode::Char('a') => app.start_add(),
                        KeyCode::Char('e') => app.start_edit_link(),
                        // NEW COMMANDS
                        KeyCode::Enter => app.cycle_current_status(),
                        KeyCode::Char('d') => app.delete_current_job(),
                        KeyCode::Char('o') => app.open_current_link(),
                        _ => {}
                    },
                    
                    // --- EDITING MODE ---
                    InputMode::Editing => match key.code {
                        KeyCode::Enter => app.submit_input(),
                        KeyCode::Esc => {
                            // Cancel input
                            app.reset_input();
                        }
                        KeyCode::Backspace => {
                            app.input_buffer.pop();
                        }
                        KeyCode::Char(c) => {
                            app.input_buffer.push(c);
                        }
                        _ => {}
                    },
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

// Simple UI function to render a box
fn ui(frame: &mut ratatui::Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(frame.size());

    // --- NEW: STATS CALCULATION ---
    let total_count = app.jobs.len();
    let interview_count = app
        .jobs
        .iter()
        .filter(|j| matches!(j.status, models::Status::Interviewing))
        .count();
    let offer_count = app
        .jobs
        .iter()
        .filter(|j| matches!(j.status, models::Status::Offer))
        .count();

    // Create a dynamic title
    let title_text = format!(
        " Career Tracker | Total: {} | Interviewing: {} | Offers: {} ",
        total_count, interview_count, offer_count
    );

    // --- LIST RENDERING ---
    let items: Vec<ListItem> = app
        .jobs
        .iter()
        .map(|job| {
            let style = match job.status {
                models::Status::Applied => Style::default().fg(Color::White),
                models::Status::Interviewing => Style::default().fg(Color::Yellow),
                models::Status::Offer => Style::default().fg(Color::Green),
                models::Status::Rejected => Style::default().fg(Color::Red),
                models::Status::Ghosted => Style::default().fg(Color::DarkGray),
            };

            let (company_width, role_width, link_width, status_width) =
                column_widths(chunks[0].width);
            let link_display = if job.post_link.is_empty() {
                "-".to_string()
            } else {
                truncate(&job.post_link, link_width)
            };
            let status_text = truncate(&format!("{:?}", job.status), status_width);
            let company_text = truncate(&job.company, company_width);
            let role_text = truncate(&job.role, role_width);

            // Using format! macro to align columns slightly
            let content = format!(
                " {:<company_width$} | {:<role_width$} | {:<link_width$} | {:<status_width$}",
                company_text,
                role_text,
                link_display,
                status_text,
                company_width = company_width,
                role_width = role_width,
                link_width = link_width,
                status_width = status_width,
            );
            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title_text)) // Use new title
        .highlight_style(
            Style::default()
                .bg(Color::White)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, chunks[0], &mut app.state);

    // --- FOOTER & POPUP (Same as before) ---
    let footer_text = match app.input_mode {
        InputMode::Normal => " 'a': Add | 'e': Edit Link | 'd': Delete | Enter: Change Status | 'o': Open Link | 'q': Quit ",
        InputMode::Editing => " Typing... Enter: Confirm | Esc: Cancel ",
    };
    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::TOP));
    frame.render_widget(footer, chunks[1]);

    if let InputMode::Editing = app.input_mode {
        let area = centered_rect(60, 20, frame.size());
        frame.render_widget(Clear, area);
        
        let title = match app.input_field {
            InputField::Company => " Enter Company Name ",
            InputField::Role => " Enter Role Title ",
            InputField::Link => match app.edit_target {
                EditTarget::Existing(_) => " Edit Job Link ",
                EditTarget::New => " Enter Job Link (optional) ",
            },
        };

        let input_block = Paragraph::new(app.input_buffer.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title(title));
            
        frame.render_widget(input_block, area);
    }
}

// Helper to center a rect in the screen
fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn truncate(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        return value.to_string();
    }
    if max_len <= 3 {
        return value.chars().take(max_len).collect::<String>();
    }
    let mut truncated = value
        .chars()
        .take(max_len.saturating_sub(3))
        .collect::<String>();
    truncated.push_str("...");
    truncated
}

fn column_widths(total_width: u16) -> (usize, usize, usize, usize) {
    let total_width = total_width as usize;
    let highlight = 3usize; // ">> "
    let separators = 9usize; // three " | "
    let leading = 1usize; // leading space before first column
    let content_width = total_width
        .saturating_sub(highlight + separators + leading);

    if content_width == 0 {
        return (0, 0, 0, 0);
    }

    let min_company = 10usize;
    let min_role = 10usize;
    let min_link = 14usize;
    let min_status = 10usize;
    let min_total = min_company + min_role + min_link + min_status;

    if content_width < min_total {
        let weights = [3usize, 3usize, 4usize, 2usize];
        let weight_sum: usize = weights.iter().sum();
        let mut company = (content_width * weights[0]) / weight_sum;
        let mut role = (content_width * weights[1]) / weight_sum;
        let mut link = (content_width * weights[2]) / weight_sum;
        let mut status = content_width.saturating_sub(company + role + link);

        company = company.max(3);
        role = role.max(3);
        link = link.max(3);
        status = status.max(3);

        let total = company + role + link + status;
        if total > content_width {
            let overflow = total - content_width;
            let reduce = overflow.min(link.saturating_sub(3));
            link = link.saturating_sub(reduce);
        }

        return (company, role, link, status);
    }

    let extra = content_width - min_total;
    let company = min_company + (extra * 3 / 10);
    let role = min_role + (extra * 3 / 10);
    let mut link = min_link + (extra * 3 / 10);
    let mut status = content_width.saturating_sub(company + role + link);

    if status < min_status {
        let deficit = min_status - status;
        let take = deficit.min(link.saturating_sub(min_link));
        link = link.saturating_sub(take);
        status = content_width.saturating_sub(company + role + link);
    }

    (company, role, link, status)
}
