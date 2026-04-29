use core::cell::RefCell;

use alloc::rc::Rc;
use embassy_sync::{blocking_mutex::{NoopMutex, raw::{CriticalSectionRawMutex, NoopRawMutex}}, channel::Channel, signal::Signal, watch::Watch};
use embassy_time::Timer;
use heapless::Vec;
use crate::{drivers::ft3168::FT3168, input_matrix::actions::InputAction};

pub mod actions;
pub mod matrix;
pub mod driver;

#[derive(PartialEq, Eq, Clone, Copy, Default, Hash)]
pub struct MatrixSquare {
    pub x: u16,
    pub y: u16,
}
impl MatrixSquare {
    pub fn new(x: u16, y: u16) -> Self {
        MatrixSquare { x, y }
    }
    pub fn direction_to(&self, other_square: MatrixSquare) -> Option<Direction> {
        match (
            other_square.x as i32 - self.x as i32,
            other_square.y as i32 - self.y as i32,
        ) {
            (0, 1) => Some(Direction::U),
            (0, -1) => Some(Direction::D),
            (1, 0) => Some(Direction::L),
            (-1, 0) => Some(Direction::R),
            _ => None,
        }
    }
}

#[derive(Eq, PartialEq, Hash)]
pub struct InputPattern {
    start_square: MatrixSquare,
    moves: Vec<Direction, { InputPattern::MAX_INPUT_LEN }>,
}
impl InputPattern {
    const MAX_INPUT_LEN: usize = 10;
    pub fn new(
        start_square: MatrixSquare,
        moves: Vec<Direction, { InputPattern::MAX_INPUT_LEN }>,
    ) -> Self {
        InputPattern {
            start_square,
            moves,
        }
    }
    pub fn from_matrix_squares(matrix_squares: &[MatrixSquare]) -> Option<Self> {
        if matrix_squares.len() < 2 || matrix_squares.len() > 10 {
            return None;
        }
        let mut moves = Vec::new();
        for i in 0..(matrix_squares.len() - 1) {
            let (current_square, next_square) = (matrix_squares[i], matrix_squares[i + 1]);
            let _ = moves.push(current_square.direction_to(next_square)?);
        }
        Some(InputPattern {
            start_square: matrix_squares.first().unwrap().clone(),
            moves,
        })
    }
    /// accepts byte string with characters u(p),d(own),l(eft),r(ight)
    pub fn from_directions(start_square: MatrixSquare, input_directions: &[u8]) -> Self {
        let mut i = 0;
        let mut moves = Vec::new();

        while i < input_directions.len() {
            let direction = match input_directions[i] {
                b'u' => Direction::U,
                b'd' => Direction::D,
                b'l' => Direction::L,
                b'r' => Direction::R,
                _ => panic!("not a valid direction"),
            };
            moves.push(direction);
            i += 1;
        }

        InputPattern {
            start_square,
            moves,
        }
    }
}

#[derive(Eq, PartialEq, Debug, Hash)]
pub enum Direction {
    U,
    D,
    L,
    R,
}

