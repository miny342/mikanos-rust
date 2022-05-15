use crate::graphics::*;
use crate::ascii::FONTS;

pub fn write_ascii(writer: &PixelWriter, x: usize, y: usize, c: char, color: &PixelColor) {
    let i = c as u8;
    if !(('!' as u8) <= i && i <= ('~' as u8)) {
        return;
    }
    for dy in 0..16 {
        for dx in 0..8 {
            if (FONTS[c as usize][dy] << dx) & 0x80 != 0 {
                writer.write(x + dx, y + dy, color);
            }
        }
    }
}
