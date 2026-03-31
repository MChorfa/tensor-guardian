use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct StatusBar {
    message: String,
    is_error: bool,
}

impl StatusBar {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            is_error: false,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            is_error: true,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let style = if self.is_error {
            Style::default()
                .fg(Color::White)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::White)
                .bg(Color::Blue)
        };

        let paragraph = Paragraph::new(self.message.clone())
            .style(style)
            .block(Block::default().borders(Borders::NONE));

        frame.render_widget(paragraph, area);
    }
}
