use board::{Board, BoardEvent, Direction};
use serde::{Deserialize, Serialize};

pub mod ai;
pub mod board;
pub mod server;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameCommands {
    Input { direction: Direction },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameUpdates {
    Ticked {
        board: Board,
        events: Vec<BoardEvent>,
    },
}
