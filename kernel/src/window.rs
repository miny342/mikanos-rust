use core::ptr::null_mut;
use core::sync::atomic::AtomicUsize;

use alloc::{vec::Vec, sync::Arc};
use alloc::vec as m_vec;
use common::writer_config::{FrameBufferConfig, PixelFormat};
use conquer_once::spin::OnceCell;
use spin::{MutexGuard, Mutex};

use crate::graphics::{PixelColor, FrameBuffer};
use crate::{println, debug};


pub struct Window {
    data: Vec<Vec<PixelColor>>,
    use_alpha: bool,
    id: usize,
    pos_x: isize,
    pos_y: isize,
    shadow_buffer: FrameBuffer,
}

impl Window {
    pub fn new(width: usize, height: usize, use_alpha: bool, pos_x: isize, pos_y: isize, fmt: PixelFormat) -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let config = FrameBufferConfig {
            frame_buffer: null_mut(),
            pixels_per_scan_line: width,
            horizontal_resolution: width,
            vertical_resolution: height,
            pixel_format: fmt,
        };
        Window {
            data: m_vec![m_vec![PixelColor { r: 0, g: 0, b: 0, a: 255}; width]; height],
            use_alpha,
            id: NEXT_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed),
            pos_x,
            pos_y,
            shadow_buffer: unsafe { FrameBuffer::new(config) }
        }
    }
    fn draw_to(&self, screen: &mut FrameBuffer) {
        if self.use_alpha {
            for (y, col) in self.data.iter().enumerate() {
                for (x, c) in col.iter().enumerate().filter(|(_x, c)| c.a != 0) {
                    let ix = (x as isize) + self.pos_x;
                    let iy = (y as isize) + self.pos_y;
                    if ix < 0 || iy < 0 {
                        continue;
                    }
                    if c.a == 255 {
                        screen.write(ix as usize, iy as usize, c);
                    } else {
                        todo!()
                    }
                }
            }
        } else {
            screen.copy(self.pos_x, self.pos_y, &self.shadow_buffer);
        }
    }
    pub fn set_use_alpha(&mut self, use_alpha: bool) {
        self.use_alpha = use_alpha
    }
    pub fn write(&mut self, x: usize, y: usize, c: &PixelColor) {
        self.data[y][x] = *c;
        self.shadow_buffer.write(x, y, c);
    }
    pub fn move_to(&mut self, pos_x: isize, pos_y: isize) {
        self.pos_x = pos_x;
        self.pos_y = pos_y;
    }
    pub fn move_relative(&mut self, diff_x: isize, diff_y: isize) {
        self.pos_x += diff_x;
        self.pos_y += diff_y;
    }
}

static WINDOW_MANAGER: OnceCell<Mutex<WindowManager>> = OnceCell::uninit();

pub struct WindowManager {
    screen_buffer: FrameBuffer,
    windows: Vec<Arc<Mutex<Window>>>,
    stack: Vec<Arc<Mutex<Window>>>,
}

impl WindowManager {
    pub fn new(screen: FrameBuffer) {
        WINDOW_MANAGER.try_init_once(|| Mutex::new(WindowManager { screen_buffer: screen, windows: Vec::new(), stack: Vec::new() })).expect("already init");
    }
    pub fn new_window(width: usize, height: usize, use_alpha: bool, pos_x: isize, pos_y: isize) -> (usize, Arc<Mutex<Window>>) {
        let mut mgr = WINDOW_MANAGER.get().unwrap().lock();
        let raw_w = Window::new(width, height, use_alpha, pos_x, pos_y, mgr.screen_buffer.fmt());
        let id = raw_w.id;
        let w = Arc::new(Mutex::new(raw_w));
        let w2 = Arc::clone(&w);
        mgr.windows.push(w);
        (id, w2)
    }
    pub fn find_window(id: usize) -> Option<Arc<Mutex<Window>>> {
        WINDOW_MANAGER.get().unwrap().lock().find_window_(id)
    }
    fn find_window_(&mut self, id: usize) -> Option<Arc<Mutex<Window>>> {
        if let Some(w) = self.windows.iter().filter(|v| v.lock().id == id).next() {
            Some(Arc::clone(&w))
        } else {
            None
        }
    }
    pub fn draw() {
        let mut mgr = WINDOW_MANAGER.get().unwrap().lock();
        let screen = unsafe { &mut *(&mut mgr.screen_buffer as *mut FrameBuffer) };
        mgr.stack.iter().map(|v| v.lock().draw_to(screen)).for_each(drop);
    }
    pub fn hide(id: usize) {
        let mut mgr = WINDOW_MANAGER.get().unwrap().lock();
        if let Some((pos, _)) = mgr.stack.iter().enumerate().filter(|(_, v)| v.lock().id == id).next() {
            mgr.stack.remove(pos);
        }
    }
    pub fn up_down(id: usize, new_height: usize) {
        let mut mgr = WINDOW_MANAGER.get().unwrap().lock();
        let mut new_height = new_height;
        if new_height > mgr.stack.len() {
            new_height = mgr.stack.len();
        }
        if let Some((old_pos, _)) = mgr.stack.iter().enumerate().filter(|(_, v)| v.lock().id == id).next() {
            mgr.stack.remove(old_pos);
            new_height -= 1;
        }
        let w = Arc::clone(&mgr.find_window_(id).expect("no window id"));
        mgr.stack.insert(new_height, w);
    }
}
