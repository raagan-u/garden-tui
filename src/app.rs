use crossterm::event::KeyEvent;
use ratatui::Frame;


use crate::{
    config::NetworkConfig, context::AppContext, ui::states::{
        network_information::NetworkInformationState, order_information::OrderDashboardState,
        swap_information::SwapDashboardState, State, StateType,
    }
};


pub struct App {
    pub context: AppContext,
    state: Box<dyn State>,
    pub should_quit: bool,
}

impl App {
    pub fn new(config: NetworkConfig) -> App {
        let context = AppContext::new("localnet", &config);
        
        App {
            context: context.clone(),
            state: Box::new(NetworkInformationState::new(context.api.quote.strategies_map)),
            should_quit: false,
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        self.state.draw(frame, &mut self.context);
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        let next_state = self.state.handle_key(key, &mut self.context);

        if let Some(state_type) = next_state {
            match state_type {
                StateType::NetworkInformation => {
                    self.state = Box::new(NetworkInformationState::new(self.context.api.quote.strategies_map.clone()));
                }
                StateType::SwapInformation => {
                    self.state = Box::new(SwapDashboardState::new());
                }
                StateType::OrderInformation => {
                    self.state = Box::new(OrderDashboardState::new());
                }
                StateType::Quit => {
                    self.should_quit = true;
                }
            }
        }
    }

    pub fn get_final_message(&self) -> Option<String> {
        self.context.exit_message.clone()
    }
}
