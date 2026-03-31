use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Chart, Dataset, Gauge, Paragraph, Row, Sparkline, Table, Tabs, Wrap},
    Frame, Terminal,
};
use std::{
    collections::VecDeque,
    io,
    time::{Duration, Instant},
};
use tensor_guardian_domain::{
    aggregates::{Accelerator, Sample},
    value_objects::{AcceleratorType, MetricType},
};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::components::{AcceleratorPanel, HistoryChart, MetricCard, StatusBar};

/// Main TUI application state
pub struct TuiApp {
    /// List of discovered accelerators
    accelerators: Vec<Accelerator>,
    /// Current samples for each accelerator
    samples: Vec<Option<Sample>>,
    /// Selected accelerator index
    selected_accel: usize,
    /// History buffer for charts ( accelerator_index -> metric_history )
    history: Vec<VecDeque<f64>>,
    /// History capacity
    history_capacity: usize,
    /// Refresh interval
    refresh_interval: Duration,
    /// Last update time
    last_update: Instant,
    /// Running flag
    running: bool,
    /// Show help
    show_help: bool,
    /// Sort mode
    sort_mode: SortMode,
    /// Event receiver
    event_rx: mpsc::Receiver<TuiEvent>,
    /// Metric collection sender
    collector_tx: mpsc::Sender<CollectorCommand>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortMode {
    ByIndex,
    ByUtilization,
    ByMemory,
}

/// Events that can occur in the TUI
#[derive(Debug)]
pub enum TuiEvent {
    /// Terminal event (keyboard, resize)
    Terminal(Event),
    /// New sample received
    NewSample(usize, Sample),
    /// Tick for UI refresh
    Tick,
    /// Quit requested
    Quit,
}

/// Commands sent to the collector
#[derive(Debug)]
pub enum CollectorCommand {
    /// Collect from specific accelerator
    Collect(usize),
    /// Collect from all accelerators
    CollectAll,
    /// Stop collecting
    Stop,
}

impl TuiApp {
    pub fn new(
        accelerators: Vec<Accelerator>,
        refresh_interval_ms: u64,
    ) -> (Self, mpsc::Sender<TuiEvent>, mpsc::Receiver<CollectorCommand>) {
        let count = accelerators.len();
        let history_capacity = 100;
        
        // Create channels
        let (event_tx, event_rx) = mpsc::channel(100);
        let (collector_tx, collector_rx) = mpsc::channel(100);
        
        let app = Self {
            accelerators,
            samples: vec![None; count],
            selected_accel: 0,
            history: vec![VecDeque::with_capacity(history_capacity); count],
            history_capacity,
            refresh_interval: Duration::from_millis(refresh_interval_ms),
            last_update: Instant::now(),
            running: true,
            show_help: false,
            sort_mode: SortMode::ByIndex,
            event_rx,
            collector_tx,
        };
        
        (app, event_tx, collector_rx)
    }

    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Start event handling
        let event_tx = self.spawn_event_handler();

        // Main loop
        let result = self.run_loop(&mut terminal).await;

        // Cleanup
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    fn spawn_event_handler(&self) -> mpsc::Sender<TuiEvent> {
        let (tx, mut rx) = mpsc::channel(100);
        let tx_clone = tx.clone();
        
        tokio::spawn(async move {
            loop {
                // Poll for events with timeout
                if event::poll(Duration::from_millis(100)).unwrap_or(false) {
                    if let Ok(event) = event::read() {
                        if tx_clone.send(TuiEvent::Terminal(event)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        let tick_tx = tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(250));
            loop {
                interval.tick().await;
                if tick_tx.send(TuiEvent::Tick).await.is_err() {
                    break;
                }
            }
        });

        tx
    }

    async fn run_loop<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        while self.running {
            // Draw UI
            terminal.draw(|f| self.draw(f))?;

            // Handle events
            if let Ok(event) = self.event_rx.try_recv() {
                match event {
                    TuiEvent::Terminal(event) => self.handle_event(event).await,
                    TuiEvent::NewSample(idx, sample) => self.handle_sample(idx, sample),
                    TuiEvent::Tick => self.update(),
                    TuiEvent::Quit => self.running = false,
                }
            }

            // Check refresh interval
            if self.last_update.elapsed() >= self.refresh_interval {
                // Request new samples
                let _ = self.collector_tx.send(CollectorCommand::CollectAll).await;
                self.last_update = Instant::now();
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        Ok(())
    }

    async fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => self.running = false,
                KeyCode::Char('h') | KeyCode::Char('?') => self.show_help = !self.show_help,
                KeyCode::Char('s') => self.cycle_sort_mode(),
                KeyCode::Up => self.select_previous(),
                KeyCode::Down => self.select_next(),
                KeyCode::Tab => self.select_next(),
                KeyCode::BackTab => self.select_previous(),
                KeyCode::Char('+') | KeyCode::Char('=') => self.increase_refresh_rate(),
                KeyCode::Char('-') => self.decrease_refresh_rate(),
                _ => {}
            },
            Event::Resize(_, _) => {}
            _ => {}
        }
    }

    fn handle_sample(&mut self, idx: usize, sample: Sample) {
        if idx < self.samples.len() {
            // Update history for utilization
            if let Some(util) = sample.utilization() {
                if idx < self.history.len() {
                    self.history[idx].push_back(util.as_percent());
                    if self.history[idx].len() > self.history_capacity {
                        self.history[idx].pop_front();
                    }
                }
            }
            self.samples[idx] = Some(sample);
        }
    }

    fn update(&mut self) {
        // Update animations, timers, etc.
    }

    fn cycle_sort_mode(&mut self) {
        self.sort_mode = match self.sort_mode {
            SortMode::ByIndex => SortMode::ByUtilization,
            SortMode::ByUtilization => SortMode::ByMemory,
            SortMode::ByMemory => SortMode::ByIndex,
        };
    }

    fn select_previous(&mut self) {
        if self.selected_accel > 0 {
            self.selected_accel -= 1;
        } else {
            self.selected_accel = self.accelerators.len().saturating_sub(1);
        }
    }

    fn select_next(&mut self) {
        if self.selected_accel + 1 < self.accelerators.len() {
            self.selected_accel += 1;
        } else {
            self.selected_accel = 0;
        }
    }

    fn increase_refresh_rate(&mut self) {
        let new_interval = self.refresh_interval.saturating_sub(Duration::from_millis(250));
        if new_interval >= Duration::from_millis(100) {
            self.refresh_interval = new_interval;
        }
    }

    fn decrease_refresh_rate(&mut self) {
        self.refresh_interval = self.refresh_interval.saturating_add(Duration::from_millis(250));
        if self.refresh_interval > Duration::from_secs(10) {
            self.refresh_interval = Duration::from_secs(10);
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(10),   // Main content
                Constraint::Length(3), // Footer
            ])
            .split(frame.area());

        // Header
        self.draw_header(frame, main_layout[0]);

        // Main content
        self.draw_main_content(frame, main_layout[1]);

        // Footer / Status bar
        self.draw_footer(frame, main_layout[2]);

        // Help overlay
        if self.show_help {
            self.draw_help(frame);
        }
    }

    fn draw_header(&self, frame: &mut Frame, area: Rect) {
        let title = format!(
            " tensor-guardian {} | {} accelerators | {}ms refresh ",
            env!("CARGO_PKG_VERSION"),
            self.accelerators.len(),
            self.refresh_interval.as_millis()
        );

        let header = Paragraph::new(title)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .alignment(Alignment::Center);

        frame.render_widget(header, area);
    }

    fn draw_main_content(&self, frame: &mut Frame, area: Rect) {
        if self.accelerators.is_empty() {
            let msg = Paragraph::new("No accelerators discovered.\n\nPress 'q' to quit.")
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center);
            frame.render_widget(msg, area);
            return;
        }

        // Split into left panel (list) and right panel (details)
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        // Draw accelerator list
        self.draw_accelerator_list(frame, layout[0]);

        // Draw selected accelerator details
        self.draw_accelerator_details(frame, layout[1]);
    }

    fn draw_accelerator_list(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<Row> = self
            .accelerators
            .iter()
            .enumerate()
            .map(|(idx, accel)| {
                let sample = self.samples.get(idx).and_then(|s| s.as_ref());
                let utilization = sample
                    .and_then(|s| s.utilization())
                    .map(|u| format!("{:.1}%", u.as_percent()))
                    .unwrap_or_else(|| "--".to_string());

                let memory = sample
                    .and_then(|s| s.memory_stats())
                    .map(|m| {
                        let used_gb = m.used_bytes as f64 / 1_073_741_824.0;
                        let total_gb = m.total_bytes as f64 / 1_073_741_824.0;
                        format!("{:.1}/{:.1} GB", used_gb, total_gb)
                    })
                    .unwrap_or_else(|| "--".to_string());

                let style = if idx == self.selected_accel {
                    Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                Row::new(vec![
                    Cell::from(format!("{}", idx)),
                    Cell::from(accel.name.clone()),
                    Cell::from(accel.backend_type.clone()),
                    Cell::from(utilization),
                    Cell::from(memory),
                ])
                .style(style)
            })
            .collect();

        let table = Table::new(
            items,
            [
                Constraint::Length(4),   // Index
                Constraint::Length(20),  // Name
                Constraint::Length(8),   // Backend
                Constraint::Length(8),   // Util
                Constraint::Length(15), // Memory
            ],
        )
        .header(
            Row::new(vec!["#", "Name", "Type", "Util", "Memory"])
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(" Accelerators ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );

        frame.render_widget(table, area);
    }

    fn draw_accelerator_details(&self, frame: &mut Frame, area: Rect) {
        if let Some(accel) = self.accelerators.get(self.selected_accel) {
            let sample = self.samples.get(self.selected_accel).and_then(|s| s.as_ref());

            // Split into metric cards and chart
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(12), Constraint::Min(5)])
                .split(area);

            // Draw metric cards
            self.draw_metric_cards(frame, layout[0], accel, sample);

            // Draw history chart
            self.draw_history_chart(frame, layout[1]);
        }
    }

    fn draw_metric_cards(&self, frame: &mut Frame, area: Rect, accel: &Accelerator, sample: Option<&Sample>) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left column: Utilization, Temperature, Power
        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Length(3)])
            .split(layout[0]);

        // GPU Utilization
        let util = sample
            .and_then(|s| s.utilization())
            .map(|u| u.as_percent())
            .unwrap_or(0.0);
        self.draw_gauge(frame, left[0], "GPU Utilization", util, Color::Green);

        // Memory
        let mem_percent = sample
            .and_then(|s| s.memory_stats())
            .map(|m| m.utilization().as_percent())
            .unwrap_or(0.0);
        self.draw_gauge(frame, left[1], "Memory", mem_percent, Color::Blue);

        // Temperature (placeholder - not in sample yet)
        self.draw_gauge(frame, left[2], "Temperature", 0.0, Color::Yellow);

        // Right column: Info
        let info_text = format!(
            "Name: {}\nType: {}\nVendor: {}\nModel: {}\nBackend: {}\nSensors: {}",
            accel.name,
            accel.accelerator_type,
            accel.vendor,
            accel.model,
            accel.backend_type,
            accel.sensors.len()
        );

        let info = Paragraph::new(info_text)
            .block(
                Block::default()
                    .title(" Info ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(info, layout[1]);
    }

    fn draw_gauge(&self, frame: &mut Frame, area: Rect, label: &str, value: f64, color: Color) {
        let gauge = Gauge::default()
            .block(Block::default().title(format!(" {} ", label)).borders(Borders::ALL))
            .gauge_style(Style::default().fg(color).bg(Color::Black))
            .percent(value as u16)
            .label(format!("{:.1}%", value));

        frame.render_widget(gauge, area);
    }

    fn draw_history_chart(&self, frame: &mut Frame, area: Rect) {
        if let Some(history) = self.history.get(self.selected_accel) {
            if history.len() >= 2 {
                let data: Vec<(f64, f64)> = history
                    .iter()
                    .enumerate()
                    .map(|(i, v)| (i as f64, *v))
                    .collect();

                let datasets = vec![Dataset::default()
                    .name("Utilization %")
                    .marker(symbols::Marker::Braille)
                    .style(Style::default().fg(Color::Cyan))
                    .graph_type(ratatui::widgets::GraphType::Line)
                    .data(&data)];

                let chart = Chart::new(datasets)
                    .block(
                        Block::default()
                            .title(" History ")
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::Blue)),
                    )
                    .x_axis(
                        ratatui::widgets::Axis::default()
                            .style(Style::default().fg(Color::Gray))
                            .bounds([0.0, self.history_capacity as f64]),
                    )
                    .y_axis(
                        ratatui::widgets::Axis::default()
                            .style(Style::default().fg(Color::Gray))
                            .bounds([0.0, 100.0])
                            .labels(["0%", "50%", "100%"]),
                    );

                frame.render_widget(chart, area);
            } else {
                let msg = Paragraph::new("Collecting data...")
                    .style(Style::default().fg(Color::Gray))
                    .alignment(Alignment::Center);
                frame.render_widget(msg, area);
            }
        }
    }

    fn draw_footer(&self, frame: &mut Frame, area: Rect) {
        let help_text = " q:Quit | h:Help | s:Sort | ↑↓:Select | +/-:Rate ";

        let footer = Paragraph::new(help_text)
            .style(Style::default().fg(Color::White).bg(Color::Blue))
            .alignment(Alignment::Center);

        frame.render_widget(footer, area);
    }

    fn draw_help(&self, frame: &mut Frame) {
        let area = frame.area();
        let popup_area = Rect {
            x: area.width / 4,
            y: area.height / 4,
            width: area.width / 2,
            height: area.height / 2,
        };

        let help_text = r#"
Keyboard Controls:

    q, Esc      Quit application
    h, ?        Toggle this help
    s           Cycle sort mode
    ↑, ↓        Select accelerator
    Tab         Next accelerator
    Shift+Tab   Previous accelerator
    +, =        Increase refresh rate
    -           Decrease refresh rate

Sort Modes:
    By Index       Default order
    By Utilization Highest first
    By Memory      Highest first

Press any key to close...
"#;

        let help = Paragraph::new(help_text)
            .block(
                Block::default()
                    .title(" Help ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });

        // Clear background
        frame.render_widget(
            Block::default().style(Style::default().bg(Color::Black)),
            area,
        );

        frame.render_widget(help, popup_area);
    }
}
