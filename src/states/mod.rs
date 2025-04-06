use ratatui::Frame;
use crossterm::event::KeyEvent;

use crate::app::AppContext;

pub mod network_selection;
pub mod network_information;
pub mod strategy_selector;

/// Enumerates the possible states the application can transition to
pub enum StateType {
    NetworkSelection,
    NetworkInformation,
    Quit,
    Exit(String), // Exit with a final message
}

/// State trait that all application states must implement
pub trait State {
    /// Draw the UI for the current state
    fn draw(&self, frame: &mut Frame, context: &mut AppContext);
    
    /// Handle key events for the current state
    /// Returns Some(StateType) if a state transition should occur, None otherwise
    fn handle_key(&self, key: KeyEvent, context: &mut AppContext) -> Option<StateType>;
}