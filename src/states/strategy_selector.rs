use std::collections::HashMap;

use ratatui::widgets::ListState;

use crate::garden_api::types::Strategy;

// First, create a struct to hold the state of the list selection
pub struct StrategySelector {
    pub state: ListState,
    pub strategies: Vec<(String, Strategy)>,
}

impl StrategySelector {
    // Create a new StrategySelector from the strategies map
    pub fn new(strategies_map: &HashMap<String, Strategy>) -> Self {
        let strategies: Vec<(String, Strategy)> = strategies_map
            .iter()
            .map(|(id, strategy)| (id.clone(), strategy.clone()))
            .collect();

        let mut state = ListState::default();
        // Select the first item by default if available
        if !strategies.is_empty() {
            state.select(Some(0));
        }

        StrategySelector { state, strategies }
    }

    // Method to select the next item
    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.strategies.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    // Method to select the previous item
    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.strategies.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    // Get the currently selected strategy (if any)
    pub fn selected_strategy(&self) -> Option<&(String, Strategy)> {
        self.state.selected().map(|i| &self.strategies[i])
    }
}
