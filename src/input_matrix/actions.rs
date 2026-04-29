use heapless::index_map::FnvIndexMap;

use crate::input_matrix::{Direction, InputPattern, MatrixSquare};

#[derive(Eq, PartialEq, Hash, Debug, Clone)]
pub enum InputAction {
    Forward,
    Back,
    Swirl,
    N3x3,
    Z3x3,
    S3x3,
    FlippedN3x3
}
impl InputAction {
    pub fn from_input_pattern(input_pattern: InputPattern) -> Option<Self> {
        use Direction::*;

        let static_input_patterns = FnvIndexMap::<_, _, 128>::from_iter([(
            InputPattern::from_directions(MatrixSquare::new(1, 1), b"rdlluurr"),
            InputAction::Swirl,
        ), (
            InputPattern::from_directions(MatrixSquare::new(1, 1), b"uurddruu"),
            InputAction::N3x3,
        ), 
        (
            InputPattern::from_directions(MatrixSquare::new(1, 1), b"rrdlldrr"),
            InputAction::Z3x3,
        ), 
        (
            InputPattern::from_directions(MatrixSquare::new(1, 1), b"lldrrdll"),
            InputAction::S3x3,
        ), 
        (
            InputPattern::from_directions(MatrixSquare::new(1, 1), b"ddruurdd"),
            InputAction::FlippedN3x3,
        ), 
        ]);
        match &input_pattern.moves[..] {
            &[R, R] => Some(InputAction::Forward),
            &[L, L] => Some(InputAction::Back),
            _ => {
                return static_input_patterns.get(&input_pattern).cloned();
            }
        }
    }
}
