use core::cell::RefCell;

use alloc::rc::Rc;
use embassy_time::Timer;
use log::info;

use crate::{
    drivers::ft3168::FT3168,
    input_matrix::{actions::InputAction, matrix::InputMatrix},
};

pub struct InputMatrixDriver {
    pub input_matrix: InputMatrix,
    pub display_shown: bool,
    pub current_input_action: Option<InputAction>,
}
impl InputMatrixDriver {
    pub fn new() -> Self {
        InputMatrixDriver {
            input_matrix: InputMatrix::new(),
            display_shown: false,
            current_input_action: None,
        }
    }
}

#[embassy_executor::task]
pub async fn input_matrix_task(
    touch_controller: Rc<RefCell<FT3168>>,
    input_matrix: Rc<RefCell<InputMatrixDriver>>,
) {
    loop {
        Timer::after_millis(100).await;
        if let Ok(touch_controller) = touch_controller.try_borrow_mut() {
            let Ok(mut input_matrix) = input_matrix.try_borrow_mut() else {
                continue;
            };
            if let Some((touch_x, touch_y)) = touch_controller.get_touch_point() {
                input_matrix.display_shown = true;
                input_matrix.input_matrix.handle_touch(touch_x, touch_y);
            } else {
                input_matrix.display_shown = false;
                if let Some(input_action) = input_matrix.input_matrix.handle_no_touch() {
                    info!("input action: {:?}", &input_action);

                    input_matrix.current_input_action = Some(input_action);
                }
            }
        }
    }
}
