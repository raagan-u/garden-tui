use crossterm::event::KeyEvent;
use ratatui::Frame;

use crate::app::AppContext;

pub mod network_information;
pub mod network_selection;
pub mod strategy_selector;
pub mod swap_information;
pub mod order_information;

pub enum StateType {
    NetworkSelection,
    NetworkInformation,
    SwapInformation,
    OrderInformation,
    Quit,
}

pub trait State {
    fn draw(&self, frame: &mut Frame, context: &mut AppContext);
    fn handle_key(&mut self, key: KeyEvent, context: &mut AppContext) -> Option<StateType>;
}