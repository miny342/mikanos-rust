use core::ptr::null_mut;
use core::sync::atomic::AtomicUsize;

use alloc::{vec::Vec, sync::Arc};
use alloc::vec as m_vec;
use common::writer_config::{FrameBufferConfig, PixelFormat};
use conquer_once::spin::OnceCell;
use spin::Mutex;

use crate::graphics::{PixelColor, FrameBuffer};
use crate::math::{Rectangle, Vector2D};
use crate::serial_println;


pub struct Window {
    data: Vec<Vec<PixelColor>>,
    use_alpha: bool,
    id: usize,
    area: Rectangle,
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
            area: Rectangle::new(Vector2D::new(pos_x, pos_y), Vector2D::new(width as isize, height as isize)),
            shadow_buffer: unsafe { FrameBuffer::new(config) }
        }
    }
    fn draw_to(&self, screen: &mut FrameBuffer) {
        if self.use_alpha {
            for (y, col) in self.data.iter().enumerate() {
                for (x, &c) in col.iter().enumerate().filter(|(_x, c)| c.a != 0) {
                    let ix = (x as isize) + self.area.pos.x;
                    let iy = (y as isize) + self.area.pos.y;
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
            screen.copy(self.area.pos.x, self.area.pos.y, &self.shadow_buffer);
        }
    }
    fn draw_rect_area_to(&self, screen: &mut FrameBuffer, r: &Rectangle) {
        if self.use_alpha {
            let r = self.area.intersect(r);
            if let Some(r) = r {
                for y in r.pos.y..r.pos.y + r.size().y {
                    for x in r.pos.x..r.pos.x + r.size().x {
                        let c = self.data[(y - self.area.pos.y) as usize][(x - self.area.pos.x) as usize];
                        if c.a == 255 {
                            screen.write(x as usize, y as usize, c);
                        } else if c.a != 0 {
                            todo!();
                        }
                    }
                }
            }
        } else {
            screen.copy_area(&self.area, &self.shadow_buffer, r);
        }
    }
    pub fn set_use_alpha(&mut self, use_alpha: bool) {
        self.use_alpha = use_alpha
    }
    pub fn write(&mut self, x: usize, y: usize, c: PixelColor) {
        self.data[y][x] = c;
        self.shadow_buffer.write(x, y, c);
    }
    pub fn move_to(&mut self, pos_x: isize, pos_y: isize) -> Rectangle {
        let old_r = self.area.clone();
        self.area.pos.x = pos_x;
        self.area.pos.y = pos_y;
        old_r
    }
    pub fn move_relative(&mut self, diff_x: isize, diff_y: isize) {
        self.area.pos.x += diff_x;
        self.area.pos.y += diff_y;
    }
    pub fn move_up_buffer(&mut self, value: usize, fill: u8) {
        self.shadow_buffer.move_up(value, fill);
    }
    pub fn write_ascii(&mut self, x: usize, y: usize, c: char, color: PixelColor) {
        let i = c as u8;
        let f = unsafe { crate::ascii::FONTS.get_unchecked(c as usize) };
        if (' ' as u8) <= i && i <= ('~' as u8) {
            for dy in 0..16 {
                for dx in 0..8 {
                    if (unsafe { f.get_unchecked(dy) } << dx) & 0x80 != 0 && 
                        x + dx < self.shadow_buffer.horizontal_resolution() &&
                        y + dy < self.shadow_buffer.vertical_resolution() {
                            self.write(x + dx, y + dy, color);
                    }
                }
            }
        }
    }
    pub fn write_string(&mut self, s: &str, color: PixelColor, start_x: usize, start_y: usize) {
        let mut pos_x = start_x;
        let mut pos_y = start_y;
        for b in s.bytes() {
            let c = b as char;
            if c == '\n' {
                pos_x = start_x;
                pos_y += 16;
            } else {
                self.write_ascii(pos_x, pos_y, c, color);
                pos_x += 8;
            }
        }
    }
    pub fn draw_rect(&mut self, r: &Rectangle, color: PixelColor) {
        for y in 0..r.size().y {
            for x in 0..r.size().x {
                self.write((r.pos.x + x) as usize, (r.pos.y + y) as usize, color);
            }
        }
    }
    pub fn draw_basic_window(&mut self, title: &str) {
        let width = self.shadow_buffer.horizontal_resolution();
        let height = self.shadow_buffer.vertical_resolution();
        self.draw_rect(&Rectangle::new(Vector2D::new(0, 0), Vector2D::new(width as isize, 22)), PixelColor::from_hex(0xc6c6c6));
        self.draw_rect(&Rectangle::new(Vector2D::new(0, 22), Vector2D::new(width as isize, height as isize - 22)), PixelColor::from_hex(0x161616));
        self.write_string(title, PixelColor::BLACK, 24, 4);
    }
    pub fn id(&self) -> usize {
        self.id
    }
}

static WINDOW_MANAGER: OnceCell<Mutex<WindowManager>> = OnceCell::uninit();

pub struct WindowManager {
    screen_buffer: FrameBuffer,
    back_buffer: FrameBuffer,
    windows: Vec<Arc<Mutex<Window>>>,
    stack: Vec<Arc<Mutex<Window>>>,
}

impl WindowManager {
    pub fn new(screen: FrameBuffer) {
        let back_buffer_config = FrameBufferConfig {
            frame_buffer: null_mut(),
            pixels_per_scan_line: screen.horizontal_resolution(),
            horizontal_resolution: screen.horizontal_resolution(),
            vertical_resolution: screen.vertical_resolution(),
            pixel_format: screen.fmt(),
        };
        let back_buffer = unsafe { FrameBuffer::new(back_buffer_config) };
        WINDOW_MANAGER.try_init_once(|| Mutex::new(WindowManager { screen_buffer: screen, back_buffer, windows: Vec::new(), stack: Vec::new() })).expect("already init");
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
        // mgrを可変参照でとった後に不変参照にできないためこうする
        // stackとscreenは独立して動き、その間mgrはロックされているのでアクセスされることはない
        let back_buffer = unsafe  { &mut *&raw mut mgr.back_buffer };
        let screen = unsafe { &mut *(&mut mgr.screen_buffer as *mut FrameBuffer) };
        mgr.stack.iter().map(|v| v.lock().draw_to(back_buffer)).for_each(drop);
        screen.copy(0, 0, &back_buffer);
    }
    pub fn draw_rect_area(r: &Rectangle) {
        let mut mgr = WINDOW_MANAGER.get().unwrap().lock();
        let back_buffer = unsafe  { &mut *&raw mut mgr.back_buffer };
        let screen = unsafe { &mut *(&mut mgr.screen_buffer as *mut FrameBuffer) };
        mgr.stack.iter().map(|v| v.lock().draw_rect_area_to(back_buffer, r)).for_each(drop);
        screen.copy_area(&back_buffer.area(Vector2D::new(0,0)), &back_buffer, r);
    }
    pub fn draw_window(id: usize) {
        let mut mgr = WINDOW_MANAGER.get().unwrap().lock();
        let back_buffer = unsafe  { &mut *&raw mut mgr.back_buffer };
        let screen = unsafe { &mut *(&mut mgr.screen_buffer as *mut FrameBuffer) };
        let mut rect = None;
        for window in mgr.stack.iter() {
            let w = window.lock();
            if w.id == id {
                rect = Some(w.area.clone());
            }
            if let Some(ref r) = rect {
                w.draw_rect_area_to(back_buffer, r);
            }
        }
        if let Some(ref r) = rect {
            screen.copy_area(&back_buffer.area(Vector2D::new(0,0)), &back_buffer, r);
        }
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
