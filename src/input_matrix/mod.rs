pub mod input;
pub mod actions;

use core::{
    cell::RefCell,
    mem::MaybeUninit,
    num::NonZeroU8,
    sync::atomic::{AtomicBool, Ordering},
};

use alloc::rc::Rc;
use embassy_time::Timer;
use heapless::index_map::FnvIndexMap;
use multiconst::multiconst;
use num_enum::TryFromPrimitive;

use crate::drivers::ft3168::FT3168;

pub static SHOULD_SHOW: AtomicBool = AtomicBool::new(false);

/// 3x3 matrix squares represented by the positions of QWE,ASD,ZXC on QWERTY keyboard
#[repr(u8)]
#[rustfmt::skip]
#[derive(Eq, PartialEq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum MatrixSquare {
    Q, W, E,
    A, S, D,
    Z, X, C
}
impl MatrixSquare {
    pub const fn from_coord(x: usize, y: usize) -> Self {
        use MatrixSquare::*;
        match (x, y) {
            (0, 0) => Q,
            (1, 0) => W,
            (2, 0) => E,
            (0, 1) => A,
            (1, 1) => S,
            (2, 1) => D,
            (0, 2) => Z,
            (1, 2) => X,
            (2, 2) => C,
            _ => panic!("not a valid coord"),
        }
    }
    pub const fn x(&self) -> usize {
        use MatrixSquare::*;
        match self {
            Q | A | Z => 0,
            W | S | X => 1,
            E | D | C => 2,
        }
    }
    pub const fn y(&self) -> usize {
        use MatrixSquare::*;
        match self {
            Q | W | E => 0,
            A | S | D => 1,
            Z | X | C => 2,
        }
    }
}

pub enum InputMatrixAction {
    Undo,
    Redo,
    /// holds inputted char in the form of byte
    CharInput(u8),
}
impl InputMatrixAction {
    pub fn from_input_pattern(input_pattern: &[MatrixSquare]) -> Option<InputMatrixAction> {
        use MatrixSquare::*;
        match input_pattern {
            &[D, S, A] => Some(InputMatrixAction::Undo),
            &[A, S, D] => Some(InputMatrixAction::Redo),
            _ => InputMatrixAction::from_character_input_pattern(input_pattern),
        }
    }
}

#[embassy_executor::task]
pub async fn input_matrix_task(touch_controller: Rc<RefCell<FT3168>>) {
    loop {
        if let Some((touch_x, touch_y)) = touch_controller.borrow_mut().get_touch_point() {
            SHOULD_SHOW.store(true, Ordering::SeqCst);
        } else {
            SHOULD_SHOW.store(false, Ordering::SeqCst);
        }
        Timer::after_millis(100).await;
    }
}
