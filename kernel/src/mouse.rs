use alloc::sync::Arc;
use conquer_once::spin::OnceCell;
use spin::{MutexGuard, Mutex};

use crate::{graphics::{
    PixelColor,
    PixelWriter
}, window::{Window, WindowManager}};

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

static CURSOR: OnceCell<Mutex<MouseCursor>> = OnceCell::uninit(); // Mutex::new(MouseCursor { pos_x: 0, pos_y: 0, erase_color: PixelColor { r: 0, g: 0, b: 0, a: 255 } });

pub struct MouseCursor {
    pos_x: usize,
    pos_y: usize,
    erase_color: PixelColor,
    window: Arc<Mutex<Window>>,
    window_id: usize
}

impl MouseCursor {
    pub fn new() -> usize {
        let (id, w) = WindowManager::new_window(12, 14, true, 0, 0);
        CURSOR.try_init_once(|| Mutex::new(
            MouseCursor {
                pos_x: 0,
                pos_y: 0,
                erase_color: PixelColor { r: 0, g: 0, b: 0, a: 0 },
                window: w,
                window_id: id,
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
                        window.write(x_idx, y_idx, PixelColor { r: 0, g: 0, b: 0, a: 255 })
                    }
                    if v & 0xc0 == 0x80 {
                        window.write(x_idx, y_idx, PixelColor { r: 255, g: 255, b: 255, a: 255 })
                    }
                    if v & 0xc0 == 0x00 {
                        window.write(x_idx, y_idx, PixelColor { r: 0, g: 0, b: 0, a: 0 })
                    }
                    v <<= 2;
                }
            }
        }
    }
    pub fn id() -> usize {
        CURSOR.try_get().unwrap().lock().window_id
    }

    // fn erase_mouse_cursor(&self, writer: &mut MutexGuard<PixelWriter>, pos_x: usize, pos_y: usize) {
    //     for col in 0..14 {
    //         for row in 0..12 {
    //             writer.write(pos_x + row, pos_y + col, &self.erase_color)
    //         }
    //     }
    // }
    // fn move_relative(&mut self, diff_x: i8, diff_y: i8) {
    //     let mut writer = PixelWriter::get().unwrap().lock();
    //     let mut x = (self.pos_x as isize) + (diff_x as isize);
    //     let mut y = (self.pos_y as isize) + (diff_y as isize);
    //     if x < 0 {
    //         x = 0;
    //     } else if x >= writer.horizontal_resolution() as isize {
    //         x = writer.horizontal_resolution() as isize - 1;
    //     }
    //     if y < 0 {
    //         y = 0;
    //     } else if y >= writer.vertical_resolution() as isize {
    //         y = writer.vertical_resolution() as isize - 1;
    //     }
    //     self.erase_mouse_cursor(&mut writer, self.pos_x, self.pos_y);
    //     self.draw_mouse_cursor(&mut writer, x as usize, y as usize);
    //     self.pos_x = x as usize;
    //     self.pos_y = y as usize;
    // }
}

pub fn mouse_handler(modifire: u8, move_x: i8, move_y: i8) {
    // mouse::CURSOR.lock().move_relative(move_x, move_y)
    CURSOR.get().unwrap().lock().window.lock().move_relative(move_x as isize, move_y as isize);
    WindowManager::draw();
}



