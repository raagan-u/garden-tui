use crossterm::event::KeyEvent;
use ratatui::Frame;

use crate::app::AppContext;

pub mod network_information;
pub mod network_selection;
pub mod strategy_selector;
pub mod swap_information;
/// Enumerates the possible states the application can transition to
pub enum StateType {
    NetworkSelection,
    NetworkInformation,
    Swapinformation,
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
