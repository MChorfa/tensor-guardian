use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Sparkline},
    Frame,
};
use std::collections::VecDeque;

pub struct HistoryChart {
    data: VecDeque<u64>,
    max_data_points: usize,
}

impl HistoryChart {
    pub fn new(max_data_points: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(max_data_points),
            max_data_points,
        }
    }

    pub fn push(&mut self, value: u64) {
        if self.data.len() >= self.max_data_points {
            self.data.pop_front();
        }
        self.data.push_back(value);
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, title: &str) {
        let data: Vec<u64> = self.data.iter().copied().collect();
        
        let sparkline = Sparkline::default()
            .block(
                Block::default()
                    .title(format!(" {} ", title))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .data(&data)
            .style(Style::default().fg(Color::Cyan))
            .max(100);

        frame.render_widget(sparkline, area);
    }
}
