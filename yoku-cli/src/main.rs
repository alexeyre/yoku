use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use dotenvy::dotenv;
use std::fmt;

use yoku_core::db::models::{DisplayableSet, Set, Workout};
use yoku_core::db::operations::{
    create_workout, delete_set, delete_workout, get_all_workouts, get_exercise,
    get_sets_for_workout,
};
use yoku_core::parser::llm::LlmInterface;
use yoku_core::session::Session;

use crossterm::event::{self, KeyCode};
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

#[derive(Parser, Debug)]
#[command(version, about = "Yoku - Workout Tracker CLI", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, ValueEnum)]
enum ParserType {
    Ollama,
    #[value(name = "openai")]
    OpenAI,
}

impl fmt::Display for ParserType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParserType::Ollama => write!(f, "ollama"),
            ParserType::OpenAI => write!(f, "openai"),
        }
    }
}

struct WorkoutSelector {
    workouts: Vec<Workout>,
    selected: usize,
    status_message: String,
    input_mode: InputMode,
    input_buffer: String,
}

enum InputMode {
    Normal,
    CreatingWorkout,
}

impl WorkoutSelector {
    async fn new() -> Result<Self> {
        let workouts = get_all_workouts().await?;
        let status_message = if workouts.is_empty() {
            "No workouts found. Press 'n' to create a new workout, 'q' to quit".to_string()
        } else {
            "j/k: navigate | n: new workout | r: resume | d: delete | q: quit".to_string()
        };

        Ok(Self {
            workouts,
            selected: 0,
            status_message,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
        })
    }

    fn scroll_down(&mut self) {
        if !self.workouts.is_empty() && self.selected < self.workouts.len() - 1 {
            self.selected += 1;
        }
    }

    fn scroll_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    fn enter_create_mode(&mut self) {
        self.input_mode = InputMode::CreatingWorkout;
        self.input_buffer.clear();
        self.status_message = "Enter workout name (or press Enter for default):".to_string();
    }

    async fn create_workout(&mut self) -> Result<()> {
        let name = if self.input_buffer.is_empty() {
            None
        } else {
            Some(self.input_buffer.clone())
        };

        let workout = create_workout(name, None).await?;
        self.workouts.push(workout.clone());
        self.selected = self.workouts.len() - 1;
        self.input_mode = InputMode::Normal;
        self.input_buffer.clear();

        let display_name = workout
            .name
            .unwrap_or_else(|| format!("Workout #{}", workout.id));
        self.status_message = format!("Created workout: {}", display_name);

        Ok(())
    }

    async fn delete_selected(&mut self) -> Result<()> {
        if self.workouts.is_empty() {
            return Ok(());
        }

        let workout = &self.workouts[self.selected];
        let workout_id = workout.id;
        let display_name = workout
            .name
            .clone()
            .unwrap_or_else(|| format!("Workout #{}", workout_id));

        delete_workout(workout_id).await?;
        self.workouts.remove(self.selected);

        if self.selected >= self.workouts.len() && self.workouts.len() > 0 {
            self.selected = self.workouts.len() - 1;
        }

        self.status_message = format!("Deleted workout: {}", display_name);
        Ok(())
    }

    fn get_selected_workout_id(&self) -> Option<i32> {
        if self.workouts.is_empty() {
            None
        } else {
            Some(self.workouts[self.selected].id)
        }
    }
}

async fn run_workout_selector(mut terminal: DefaultTerminal) -> Result<Option<i32>> {
    let mut selector = WorkoutSelector::new().await?;

    loop {
        terminal.draw(|frame| {
            let chunks = Layout::vertical([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(frame.area());

            // Header
            let header = Paragraph::new("Yoku - Workout Tracker")
                .style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(header, chunks[0]);

            // Workout list or input prompt
            match selector.input_mode {
                InputMode::Normal => {
                    if selector.workouts.is_empty() {
                        let empty_msg = Paragraph::new(
                            "No workouts found.\nPress 'n' to create your first workout!",
                        )
                        .style(Style::default().fg(Color::Gray))
                        .block(Block::default().borders(Borders::ALL).title("Workouts"));
                        frame.render_widget(empty_msg, chunks[1]);
                    } else {
                        let items: Vec<ListItem> = selector
                            .workouts
                            .iter()
                            .enumerate()
                            .map(|(idx, workout)| {
                                let display_name = workout
                                    .name
                                    .clone()
                                    .unwrap_or_else(|| format!("Workout #{}", workout.id));

                                let date_str = workout
                                    .performed_at
                                    .map(|dt| format!(" - {}", dt.format("%Y-%m-%d %H:%M")))
                                    .unwrap_or_default();

                                let content = format!("{}{}", display_name, date_str);

                                let style = if idx == selector.selected {
                                    Style::default()
                                        .fg(Color::Black)
                                        .bg(Color::Cyan)
                                        .add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default()
                                };

                                ListItem::new(content).style(style)
                            })
                            .collect();

                        let list = List::new(items).block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title(format!("Workouts ({} total)", selector.workouts.len())),
                        );

                        let mut list_state = ListState::default();
                        list_state.select(Some(selector.selected));

                        frame.render_stateful_widget(list, chunks[1], &mut list_state);
                    }
                }
                InputMode::CreatingWorkout => {
                    let input_widget = Paragraph::new(selector.input_buffer.as_str())
                        .style(Style::default().fg(Color::Yellow))
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title("New Workout Name"),
                        );
                    frame.render_widget(input_widget, chunks[1]);
                }
            }

            // Footer with status
            let footer = Paragraph::new(selector.status_message.as_str())
                .style(Style::default().fg(Color::White))
                .block(Block::default().borders(Borders::ALL).title("Status"));
            frame.render_widget(footer, chunks[2]);
        })?;

        if let event::Event::Key(key) = event::read()? {
            match selector.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        return Ok(None);
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        selector.scroll_down();
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        selector.scroll_up();
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        selector.enter_create_mode();
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') => {
                        selector.delete_selected().await?;
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        if let Some(workout_id) = selector.get_selected_workout_id() {
                            return Ok(Some(workout_id));
                        }
                    }
                    _ => {}
                },
                InputMode::CreatingWorkout => match key.code {
                    KeyCode::Enter => {
                        selector.create_workout().await?;
                    }
                    KeyCode::Esc => {
                        selector.input_mode = InputMode::Normal;
                        selector.input_buffer.clear();
                        selector.status_message =
                            "j/k: navigate | n: new workout | r: resume | d: delete | q: quit"
                                .to_string();
                    }
                    KeyCode::Char(c) => {
                        selector.input_buffer.push(c);
                    }
                    KeyCode::Backspace => {
                        selector.input_buffer.pop();
                    }
                    _ => {}
                },
            }
        }
    }
}

struct WorkoutSession<T: LlmInterface> {
    workout_id: i32,
    sets: Vec<(Set, String)>, // (Set, exercise_name)
    selected: usize,
    status_message: String,
    input_mode: SessionInputMode,
    input_buffer: String,
    parser: Box<T>,
    session: Session,
}

enum SessionInputMode {
    Normal,
    CreatingSet,
}

impl<T: LlmInterface> WorkoutSession<T> {
    async fn new(workout_id: i32, parser: Box<T>) -> Result<Self> {
        let mut session = Session::new_blank().await;
        session.set_workout_id(workout_id).await?;

        let mut workout_session = Self {
            workout_id,
            sets: Vec::new(),
            selected: 0,
            status_message: "c: create set | d: delete | q: quit".to_string(),
            input_mode: SessionInputMode::Normal,
            input_buffer: String::new(),
            parser,
            session,
        };

        workout_session.refresh_sets().await?;
        Ok(workout_session)
    }

    async fn refresh_sets(&mut self) -> Result<()> {
        let sets = get_sets_for_workout(self.workout_id).await?;
        self.sets.clear();

        for set in sets {
            let exercise = get_exercise(set.exercise_id).await?;
            self.sets.push((set, exercise.name));
        }

        if self.selected >= self.sets.len() && self.sets.len() > 0 {
            self.selected = self.sets.len() - 1;
        }

        Ok(())
    }

    fn scroll_down(&mut self) {
        if !self.sets.is_empty() && self.selected < self.sets.len() - 1 {
            self.selected += 1;
        }
    }

    fn scroll_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    fn enter_create_mode(&mut self) {
        self.input_mode = SessionInputMode::CreatingSet;
        self.input_buffer.clear();
        self.status_message =
            "Enter set description (e.g., 'bench press 100kg x 5 reps'):".to_string();
    }

    async fn create_set(&mut self) -> Result<()> {
        let parsed_set = self.parser.parse_set_string(&self.input_buffer).await?;
        self.session.add_set_from_parsed(&parsed_set).await?;

        self.input_mode = SessionInputMode::Normal;
        self.input_buffer.clear();
        self.status_message = format!("Added set: {}", parsed_set.exercise);

        self.refresh_sets().await?;
        Ok(())
    }

    async fn delete_selected(&mut self) -> Result<()> {
        if self.sets.is_empty() {
            return Ok(());
        }

        let (set, exercise_name) = &self.sets[self.selected];
        let set_id = set.id;

        delete_set(set_id).await?;
        self.status_message = format!("Deleted {} set", exercise_name);

        self.refresh_sets().await?;
        Ok(())
    }
}

async fn run_workout_session<T: LlmInterface>(
    mut terminal: DefaultTerminal,
    workout_id: i32,
    parser: Box<T>,
) -> Result<()> {
    let mut session = WorkoutSession::new(workout_id, parser).await?;

    loop {
        terminal.draw(|frame| {
            let chunks = Layout::vertical([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(frame.area());

            // Header
            let header = Paragraph::new(format!("Workout #{}", workout_id))
                .style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(header, chunks[0]);

            // Sets list or input prompt
            match session.input_mode {
                SessionInputMode::Normal => {
                    if session.sets.is_empty() {
                        let empty_msg = Paragraph::new(
                            "No sets recorded yet.\nPress 'c' to create your first set!",
                        )
                        .style(Style::default().fg(Color::Gray))
                        .block(Block::default().borders(Borders::ALL).title("Sets"));
                        frame.render_widget(empty_msg, chunks[1]);
                    } else {
                        let items: Vec<ListItem> = session
                            .sets
                            .iter()
                            .enumerate()
                            .map(|(idx, (set, exercise_name))| {
                                let rpe_str =
                                    set.rpe.map(|r| format!(" @{:.1}", r)).unwrap_or_default();
                                let content = format!(
                                    "{}: {:.1}kg x {} reps{}",
                                    exercise_name, set.weight, set.reps, rpe_str
                                );

                                let style = if idx == session.selected {
                                    Style::default()
                                        .fg(Color::Black)
                                        .bg(Color::Cyan)
                                        .add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default()
                                };

                                ListItem::new(content).style(style)
                            })
                            .collect();

                        let list = List::new(items).block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title(format!("Sets ({} total)", session.sets.len())),
                        );

                        let mut list_state = ListState::default();
                        list_state.select(Some(session.selected));

                        frame.render_stateful_widget(list, chunks[1], &mut list_state);
                    }
                }
                SessionInputMode::CreatingSet => {
                    let input_widget = Paragraph::new(session.input_buffer.as_str())
                        .style(Style::default().fg(Color::Yellow))
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title("New Set Description"),
                        );
                    frame.render_widget(input_widget, chunks[1]);
                }
            }

            // Footer with status
            let footer = Paragraph::new(session.status_message.as_str())
                .style(Style::default().fg(Color::White))
                .block(Block::default().borders(Borders::ALL).title("Status"));
            frame.render_widget(footer, chunks[2]);
        })?;

        if let event::Event::Key(key) = event::read()? {
            match session.input_mode {
                SessionInputMode::Normal => match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        return Ok(());
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        session.scroll_down();
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        session.scroll_up();
                    }
                    KeyCode::Char('c') | KeyCode::Char('C') => {
                        session.enter_create_mode();
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') => {
                        session.delete_selected().await?;
                    }
                    _ => {}
                },
                SessionInputMode::CreatingSet => match key.code {
                    KeyCode::Enter => {
                        session.status_message =
                            format!("Processing set string: {}", session.input_buffer);
                        if let Err(e) = session.create_set().await {
                            session.status_message = format!("Error creating set: {}", e);
                            session.input_mode = SessionInputMode::Normal;
                            session.input_buffer.clear();
                        }
                    }
                    KeyCode::Esc => {
                        session.input_mode = SessionInputMode::Normal;
                        session.input_buffer.clear();
                        session.status_message = "c: create set | d: delete | q: quit".to_string();
                    }
                    KeyCode::Char(c) => {
                        session.input_buffer.push(c);
                    }
                    KeyCode::Backspace => {
                        session.input_buffer.pop();
                    }
                    _ => {}
                },
            }
        }
    }
}

struct App<T: LlmInterface> {
    parser: Box<T>,
    session: Session,
}

impl<T: LlmInterface> App<T> {
    async fn parse_input(&self, input: &str) -> Result<()> {
        let parsed_set = self.parser.parse_set_string(input).await?;
        println!("Parsed set: {:?}", parsed_set);
        Ok(())
    }
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Interactive workout selection and tracking
    Interactive {
        #[arg(short, long, default_value_t = ParserType::Ollama)]
        parser: ParserType,
        #[arg(short, long)]
        model: Option<String>,
    },
    Parse {
        #[arg(short, long)]
        input: String,
        #[arg(short, long, default_value_t = ParserType::Ollama)]
        parser: ParserType,
        #[arg(short, long)]
        model: Option<String>,
    },
    /// List all workouts from the database
    ListWorkouts {
        #[arg(short, long)]
        verbose: bool,
    },
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> Result<()> {
    dotenv().ok();
    let args = Args::parse();

    match args.command {
        Commands::Interactive { parser, model } => {
            let terminal = ratatui::init();
            let result = run_workout_selector(terminal).await;
            ratatui::restore();

            match result {
                Ok(Some(workout_id)) => {
                    println!("Selected workout {}. Starting session...\n", workout_id);

                    // Create parser based on user selection
                    let terminal = ratatui::init();
                    let session_result = match parser {
                        ParserType::Ollama => {
                            let parser = yoku_core::parser::llm::Ollama::new(model).await?;
                            run_workout_session(terminal, workout_id, parser).await
                        }
                        ParserType::OpenAI => {
                            let parser = yoku_core::parser::llm::OpenAi::new(model).await?;
                            run_workout_session(terminal, workout_id, parser).await
                        }
                    };
                    ratatui::restore();
                    session_result
                }
                Ok(None) => {
                    println!("Exited without selecting a workout");
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
        Commands::Parse {
            input,
            parser,
            model,
        } => {
            match parser {
                ParserType::Ollama => {
                    let parser = yoku_core::parser::llm::Ollama::new(model).await?;
                    let app = App {
                        parser,
                        session: Session::new_blank().await,
                    };
                    app.parse_input(&input).await?;
                }
                ParserType::OpenAI => {
                    let parser = yoku_core::parser::llm::OpenAi::new(model).await?;
                    let app = App {
                        parser,
                        session: Session::new_blank().await,
                    };
                    app.parse_input(&input).await?;
                }
            }
            Ok(())
        }
        Commands::ListWorkouts { verbose } => {
            let workouts = get_all_workouts().await?;
            for workout in workouts {
                println!(
                    "{}, {}",
                    workout.id,
                    workout.name.unwrap_or("Unnamed Workout".into())
                );
                if verbose {
                    let sets = get_sets_for_workout(workout.id).await?;
                    for set in sets {
                        let exercise = get_exercise(set.exercise_id).await?;
                        let displayable = DisplayableSet::new(set, exercise.name);
                        println!("\t{}", displayable)
                    }
                }
            }
            Ok(())
        }
    }
}
