use crossterm::event::KeyEvent;
use ratatui::Frame;

use crate::app::AppContext;

use super::{State, StateType};

pub struct SwapDashboardState;

impl SwapDashboardState {
    pub fn new() -> Self {
        SwapDashboardState {}
    }
}
impl State for SwapDashboardState {
    /// Draw the UI for the current state
    fn draw(&self, frame: &mut Frame, context: &mut AppContext){
        
    }
    
    /// Handle key events for the current state
    /// Returns Some(StateType) if a state transition should occur, None otherwise
    fn handle_key(&self, key: KeyEvent, context: &mut AppContext) -> Option<StateType> {
        todo!()
    }
}