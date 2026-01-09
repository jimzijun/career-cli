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
use models::Status; // Import Status to match against it
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
                // Create the job
                let new_id = self.jobs.len() + 1;
                let new_job = Job::new(
                    new_id, 
                    self.temp_company.clone(), 
                    self.input_buffer.clone()
                );
                self.jobs.push(new_job);
                
                // Reset state
                self.input_buffer.clear();
                self.temp_company.clear();
                self.input_mode = InputMode::Normal;
                self.input_field = InputField::Company; // Reset for next time
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
                        KeyCode::Char('a') => {
                            app.input_mode = InputMode::Editing;
                            app.input_field = InputField::Company;
                        }
                        // NEW COMMANDS
                        KeyCode::Enter => app.cycle_current_status(),
                        KeyCode::Char('d') => app.delete_current_job(),
                        _ => {}
                    },
                    
                    // --- EDITING MODE ---
                    InputMode::Editing => match key.code {
                        KeyCode::Enter => app.submit_input(),
                        KeyCode::Esc => {
                            // Cancel input
                            app.input_mode = InputMode::Normal;
                            app.input_buffer.clear();
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

            // Using format! macro to align columns slightly
            let content = format!(" {:<20} | {:<20} | {:?}", job.company, job.role, job.status);
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
        InputMode::Normal => " 'a': Add | 'd': Delete | Enter: Change Status | 'q': Quit ",
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