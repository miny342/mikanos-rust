use spin::MutexGuard;

use crate::graphics::*;
use crate::ascii::FONTS;
use crate::window::Window;

pub fn write_ascii(writer: &mut MutexGuard<Window>, x: usize, y: usize, c: char, color: &PixelColor) {
    let i = c as u8;
    let f = unsafe { FONTS.get_unchecked(c as usize) };
    if (' ' as u8) <= i && i <= ('~' as u8) {
        for dy in 0..16 {
            for dx in 0..8 {
                if (unsafe { f.get_unchecked(dy) } << dx) & 0x80 != 0 {
                    writer.write(x + dx, y + dy, color);
                }
            }
        }
    }
}
