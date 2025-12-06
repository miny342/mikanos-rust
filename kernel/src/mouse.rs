use alloc::sync::Arc;
use conquer_once::spin::OnceCell;
use log::{debug, warn};
use spin::Mutex;

use crate::{graphics::PixelColor, math::Vector2D, serial_println, timer::check_time, window::{Window, WindowID, WindowManager}};

const MOUSE_CURSOR: [[u8; 3]; 14] = [
    [64, 0, 0],
    [80, 0, 0],
    [100, 0, 0],
    [105, 0, 0],
    [106, 64, 0],
    [106, 144, 0],
    [106, 164, 0],
    [106, 169, 0],
    [106, 170, 64],
    [106, 170, 144],
    [106, 170, 164],
    [106, 165, 85],
    [105, 80, 0],
    [84, 0, 0]
];

#[bitfield_struct::bitfield(u8)]
struct Modifire {
    left_pressed: bool,
    right_pressed: bool,
    center_pressed: bool,
    #[bits(5)]
    _reserved: u8,
}

static CURSOR: OnceCell<Mutex<MouseCursor>> = OnceCell::uninit();

pub struct MouseCursor {
    pos_x: usize,
    pos_y: usize,
    screen_x: usize,
    screen_y: usize,
    window: Arc<Mutex<Window>>,
    window_id: WindowID,
    prev_modifire: Modifire,
    dragging_window: Option<Arc<Mutex<Window>>>,
}

impl MouseCursor {
    pub fn new() -> WindowID {
        let (id, w) = WindowManager::new_window(12, 14, true, 0, 0, false);
        let (screen_x, screen_y) = WindowManager::resolution();
        CURSOR.try_init_once(|| Mutex::new(
            MouseCursor {
                pos_x: 0,
                pos_y: 0,
                screen_x,
                screen_y,
                window: w,
                window_id: id,
                prev_modifire: Modifire(0),
                dragging_window: None,
            }
        )).unwrap();
        Self::draw_mouse_cursor(&CURSOR.try_get().unwrap().lock());
        id
    }
    fn draw_mouse_cursor(&self) {
        let mut window = self.window.lock();
        for (col, col_val) in MOUSE_CURSOR.iter().enumerate() {
            for (row, row_val) in col_val.iter().enumerate() {
                let mut v = *row_val;
                for i in 0..4 {
                    let x_idx = row * 4 + i;
                    let y_idx = col;
                    if v & 0xc0 == 0x40 {
                        window.write(x_idx, y_idx, PixelColor::BLACK)
                    }
                    if v & 0xc0 == 0x80 {
                        window.write(x_idx, y_idx, PixelColor::WHITE)
                    }
                    if v & 0xc0 == 0x00 {
                        window.write(x_idx, y_idx, PixelColor::TRANSPARENT)
                    }
                    v <<= 2;
                }
            }
        }
    }
    pub fn id() -> WindowID {
        CURSOR.try_get().unwrap().lock().window_id
    }
}

pub fn mouse_handler(modifire: u8, move_x: i8, move_y: i8) {
    log::set_max_level(log::LevelFilter::Debug);
    let mut mouse = CURSOR.get().unwrap().lock();
    let mut x = (mouse.pos_x as isize) + (move_x as isize);
    let mut y = (mouse.pos_y as isize) + (move_y as isize);
    if x < 0 {
        x = 0;
    } else if x >= mouse.screen_x as isize {
        x = mouse.screen_x as isize - 1;
    }
    if y < 0 {
        y = 0;
    } else if y >= mouse.screen_y as isize {
        y = mouse.screen_y as isize - 1;
    }
    let diff_x = x - mouse.pos_x as isize;
    let diff_y = y - mouse.pos_y as isize;

    mouse.pos_x = x as usize;
    mouse.pos_y = y as usize;

    let modifire = Modifire(modifire);
    debug!("modifire: {:?}", modifire);
    if !mouse.prev_modifire.left_pressed() && modifire.left_pressed() {
        let w = WindowManager::find_window_by_position(&Vector2D::new(x, y), Some(mouse.window_id));
        if let Some(ref tmp) = w && tmp.lock().draggable() {
            mouse.dragging_window = w;
        }
    } else if mouse.prev_modifire.left_pressed() && modifire.left_pressed() {
        if let Some(ref w) = mouse.dragging_window {
            let (w_old_r, w_id) = {
                let mut lck = w.lock();
                (lck.move_relative(diff_x, diff_y), lck.id())
            };
            let t = check_time(|| {
                WindowManager::draw_rect_area(&w_old_r);
                WindowManager::draw_window(w_id);
            });
            debug!("dragging elapsed: {:?}", t);
        }
    } else if mouse.prev_modifire.left_pressed() && !modifire.left_pressed() {
        mouse.dragging_window = None;
    }

    mouse.prev_modifire = modifire;
    let (old_r, id) = {
        let mut lck = mouse.window.lock();
        (lck.move_to(x, y), lck.id())
    };
    let t = check_time(|| {
        WindowManager::draw_rect_area(&old_r);
        WindowManager::draw_window(id);
    });
    debug!("elapsed: {:?}", t);
    log::set_max_level(log::LevelFilter::Warn);
}
