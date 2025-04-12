use crossterm::event::KeyEvent;
use ratatui::Frame;

use crate::context::AppContext;

pub mod network_information;
pub mod swap_information;
pub mod order_information;

pub enum StateType {
    NetworkInformation,
    SwapInformation,
    OrderInformation,
    Quit,
}

pub trait State {
    fn draw(&self, frame: &mut Frame, context: &mut AppContext);
    fn handle_key(&mut self, key: KeyEvent, context: &mut AppContext) -> Option<StateType>;
}