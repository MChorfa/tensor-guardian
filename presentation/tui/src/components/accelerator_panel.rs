use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};
use tensor_guardian_domain::aggregates::Accelerator;

pub struct AcceleratorPanel;

impl AcceleratorPanel {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        accelerators: &[Accelerator],
        selected: usize,
    ) {
        if accelerators.is_empty() {
            let paragraph = Paragraph::new("No accelerators found")
                .style(Style::default().fg(Color::Yellow))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Accelerators "),
                );
            frame.render_widget(paragraph, area);
            return;
        }

        let rows: Vec<Row> = accelerators
            .iter()
            .enumerate()
            .map(|(idx, accel)| {
                let style = if idx == selected {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };

                Row::new(vec![
                    Cell::from(format!("{}", idx)),
                    Cell::from(accel.name.clone()),
                    Cell::from(format!("{}", accel.accelerator_type)),
                ])
                .style(style)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                ratatui::layout::Constraint::Length(4),
                ratatui::layout::Constraint::Length(20),
                ratatui::layout::Constraint::Length(15),
            ],
        )
        .header(Row::new(vec!["#", "Name", "Type"]).style(Style::default().fg(Color::Yellow)))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Accelerators "),
        );

        frame.render_widget(table, area);
    }
}
