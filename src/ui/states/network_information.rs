use std::collections::HashMap;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout}, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, List, ListItem, Paragraph}, Frame
};
use crossterm::event::{KeyCode, KeyEvent};

use crate::{context::AppContext, service::garden::types::Strategy, ui::components::selector::Selector};
use super::{State, StateType};
pub struct NetworkInformationState {
    order_pair_selector: Selector<Strategy>
}


impl NetworkInformationState {
    pub fn new(strategies_map: HashMap<String, Strategy>) -> Self {
        let selector = Selector::new(&strategies_map);

        NetworkInformationState {
            order_pair_selector: selector
        }
    }
}

impl State for NetworkInformationState {
    fn draw(&self, frame: &mut Frame, context: &mut AppContext) {
        let size = frame.area();

        // Create common elements
        let title_span = Span::styled(
            "Garden-TUI 0.0.1",
            Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)
        );
        let title_line = Line::from(vec![title_span]);

        let title_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White));

        // Create layout with more explicit constraints
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),   // Title
                Constraint::Length(5),   // Network Info - increased height
                Constraint::Length(10),  // Strategy Selector
                Constraint::Length(3),   // Selected Strategy (if any)
                Constraint::Min(0),      // Instructions
            ].as_ref())
            .split(size);

        // Title
        frame.render_widget(
            Paragraph::new(vec![title_line])
                .block(title_block)
                .alignment(Alignment::Center),
            chunks[0],
        );

        // Network Information with Strategy Selector
        if !context.selected_network.is_empty() {
            // Network info
            let info_text = vec![
                Line::from(vec![
                    Span::styled(format!("Selected Network: "),
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::styled(context.selected_network.clone(), Style::default().fg(Color::White))
                ]),
                Line::from(vec![
                    Span::styled("Quote Server URL: ",
                        Style::default().fg(Color::Yellow)),
                    Span::styled(context.api.quote.url.clone(), Style::default().fg(Color::White))
                ]),
            ];

            let info_block = Block::default()
                .title("Network Information")
                .borders(Borders::ALL);

            frame.render_widget(
                Paragraph::new(info_text)
                    .block(info_block)
                    .alignment(Alignment::Left),
                chunks[1],
            );


            // Render the strategy selector
           
                if !self.order_pair_selector.items.is_empty() {
                    let items: Vec<ListItem> = self.order_pair_selector.items
                        .iter()
                        .map(|(id, strategy)| {
                            ListItem::new(format!("{}: {} to {}",
                                id,
                                strategy.source_chain,
                                strategy.dest_chain))
                        })
                        .collect();

                    let list = List::new(items)
                        .block(Block::default().title("Select Strategy").borders(Borders::ALL))
                        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                        .highlight_symbol("> ");

                    frame.render_stateful_widget(list, chunks[2], &mut self.order_pair_selector.state.clone());

                    // Show currently selected strategy if any
                    if let Some(strategy_id) = context.order.current_strategy.as_ref() {
                        let selected_text = vec![
                            Line::from(vec![
                                Span::styled("Selected Strategy: ",
                                    Style::default().fg(Color::Yellow)),
                                Span::styled(strategy_id, Style::default().fg(Color::Green))
                            ])
                        ];

                        frame.render_widget(
                            Paragraph::new(selected_text)
                                .block(Block::default().borders(Borders::ALL)),
                            chunks[3],
                        );
                    }
                } else {
                    // Show message if no strategies available
                    frame.render_widget(
                        Paragraph::new("No strategies available")
                            .block(Block::default().title("Select Strategy").borders(Borders::ALL))
                            .alignment(Alignment::Center),
                        chunks[2],
                    );
                }
           
        } else {
            eprintln!("Network or URLs not selected");
        }

        // Instructions
        let instructions_spans = vec![
            Span::styled("↑/↓: Navigate | ", Style::default().fg(Color::Red)),
            Span::styled("Enter: Select Strategy | ", Style::default().fg(Color::Red)),
            Span::styled("b: Back | ", Style::default().fg(Color::Red)),
            Span::styled("q: Quit", Style::default().fg(Color::Red)),
        ];

        frame.render_widget(
            Paragraph::new(vec![Line::from(instructions_spans)])
                .alignment(Alignment::Center),
            chunks[4],
        );
    }

    fn handle_key(&mut self, key: KeyEvent, context: &mut AppContext) -> Option<StateType> {
        match key.code {
            KeyCode::Char('q') => Some(StateType::Quit),
            KeyCode::Up => {
                self.order_pair_selector.previous();
                None
            },
            KeyCode::Down => {
                    self.order_pair_selector.next();
                None
            },
            KeyCode::Enter => {
                if let Some((id, _)) = self.order_pair_selector.selected_item() {
                    context.order.current_strategy = Some(id.clone());
                }
                Some(StateType::SwapInformation)
            },
            _ => None,
        }
    }
}
