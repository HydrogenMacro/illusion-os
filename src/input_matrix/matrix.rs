use heapless::Vec;
use log::info;

use crate::{display::drivers::co5300::CO5300, input_matrix::{InputPattern, MatrixSquare, actions::InputAction}};

pub struct InputMatrix {
    currently_inputted_squares: Vec<MatrixSquare, { InputPattern::MAX_INPUT_LEN }>,
    border_size: u16,
    gap_size: u16,
    squares_per_row: u16,
    squares_per_col: u16,
}
impl InputMatrix {
    pub fn new() -> Self {
        InputMatrix {
            currently_inputted_squares: Vec::new(),
            border_size: 40,
            gap_size: 20,
            squares_per_row: 3,
            squares_per_col: 4,
        }
    }
    fn get_matrix_square_from_touch_coords(
        &self,
        touch_x: u16,
        touch_y: u16,
    ) -> Option<MatrixSquare> {
        assert!(touch_x < CO5300::WIDTH && touch_y < CO5300::HEIGHT);
        let matrix_square_width =
            (CO5300::WIDTH - self.border_size * 2 - (self.gap_size - 1) * self.squares_per_row)
                / self.squares_per_row;
        let matrix_square_height =
            (CO5300::HEIGHT - self.border_size * 2 - (self.gap_size - 1) * self.squares_per_col)
                / self.squares_per_col;
        if touch_x < self.border_size || touch_x > CO5300::WIDTH - self.border_size {
            return None;
        }
        if touch_y < self.border_size || touch_y > CO5300::HEIGHT - self.border_size {
            return None;
        }
        if (touch_x - self.border_size) % (self.squares_per_row + self.gap_size)
            < matrix_square_width
            && (touch_y - self.border_size) % (self.squares_per_col + self.gap_size)
                < matrix_square_height
        {
            return Some(MatrixSquare::new(
                (touch_x - self.border_size) / (self.squares_per_row + self.gap_size),
                (touch_y - self.border_size) / (self.squares_per_col + self.gap_size),
            ));
        } else {
            return None;
        }
    }
    pub fn handle_touch(&mut self, touch_x: u16, touch_y: u16) {
        info!("touch: {touch_x}, {touch_y}");
        if let Some(matrix_square) = self.get_matrix_square_from_touch_coords(touch_x, touch_y)
            && self
                .currently_inputted_squares
                .last()
                .is_none_or(|last_square| &matrix_square != last_square)
        {
            self.currently_inputted_squares.push(matrix_square);
        }
    }
    pub fn handle_no_touch(&mut self) -> Option<InputAction> {
        if let Some(input_action) =
            InputPattern::from_matrix_squares(&self.currently_inputted_squares)
                .and_then(|input_pattern| InputAction::from_input_pattern(input_pattern))
        {
            info!("made input action {:?}", &input_action);
            return Some(input_action);
        }
        return None;
    }
}
