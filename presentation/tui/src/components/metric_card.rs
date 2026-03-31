use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

pub struct MetricCard {
    title: String,
    value: f64,
    unit: String,
    max_value: f64,
}

impl MetricCard {
    pub fn new(title: impl Into<String>, value: f64, unit: impl Into<String>, max_value: f64) -> Self {
        Self {
            title: title.into(),
            value,
            unit: unit.into(),
            max_value,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let percentage = if self.max_value > 0.0 {
            (self.value / self.max_value * 100.0).min(100.0) as u16
        } else {
            0
        };

        let color = if percentage > 90 {
            Color::Red
        } else if percentage > 60 {
            Color::Yellow
        } else {
            Color::Green
        };

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .title(format!(" {} ", self.title))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(color)),
            )
            .gauge_style(Style::default().fg(color).bg(Color::Black))
            .percent(percentage)
            .label(format!("{:.1} {}", self.value, self.unit));

        frame.render_widget(gauge, area);
    }
}
