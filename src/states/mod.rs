use crossterm::event::KeyEvent;
use ratatui::Frame;

use crate::app::AppContext;

pub mod network_information;
pub mod network_selection;
pub mod strategy_selector;
pub mod swap_information;
pub mod order_information;
/// Enumerates the possible states the application can transition to
pub enum StateType {
    NetworkSelection,
    NetworkInformation,
    SwapInformation,
    OrderInformation,
    Quit,
}

pub trait State {
    // Draw method remains the same
    fn draw(&self, frame: &mut Frame, context: &mut AppContext);
    
    // Sync key handler remains the same
    fn handle_key(&mut self, key: KeyEvent, context: &mut AppContext) -> Option<StateType>;
    
    // Updated async key handler with corrected lifetime parameters
    fn handle_key_async<'a>(
        &'a mut self,
        key: KeyEvent,
        context: &'a mut AppContext
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<StateType>> + 'a>> {
        // Default implementation returns None
        Box::pin(async { None })
    }
}