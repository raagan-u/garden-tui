use std::collections::HashMap;
use ratatui::widgets::ListState;


pub struct Selector<T> {
    pub state: ListState,
    pub items: Vec<(String, T)>,
}

impl<T: Clone> Selector<T> {
    pub fn new(items_map: &HashMap<String, T>) -> Self {
        let items: Vec<(String, T)> = items_map
            .iter()
            .map(|(id, item)| (id.clone(), item.clone()))
            .collect();
        
        let mut state = ListState::default();

        if !items.is_empty() {
            state.select(Some(0));
        }
        
        Selector { state, items }
    }
    

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
    

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
    

    pub fn selected_item(&self) -> Option<&(String, T)> {
        self.state.selected().map(|i| &self.items[i])
    }
}