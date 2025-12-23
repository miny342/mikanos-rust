use crate::print;

const LCTRL: u8 = 1;
const LSHIFT: u8 = 1 << 1;
const LALT: u8 = 1 << 2;
const LGUI: u8 = 1 << 3;
const RCTRL: u8 = 1 << 4;
const RSHIFT: u8 = 1 << 5;
const RALT: u8 = 1 << 6;
const RGUI: u8 = 1 << 7;

const KEYCODE: [u8; 104] = [
    0, 0, 0, 0, b'a', b'b', b'c', b'd',
    b'e', b'f', b'g', b'h', b'i', b'j', b'k', b'l',
    b'm', b'n', b'o', b'p', b'q', b'r', b's', b't',
    b'u', b'v', b'w', b'x', b'y', b'z', b'1', b'2',
    b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0',
    b'\n', 0x1b, 0x7f, b'\t', b' ', b'-', b'=', b'[',
    b']', b'\\', b'#', b';', b'\'', b'`', b',', b'.',
    b'/', 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, b'/', b'*', b'-', b'+',
    b'\n', b'1', b'2', b'3', b'4', b'5', b'6', b'7',
    b'8', b'9', b'0', b'.', b'\\', 0, 0, b'=',
];

const KEYCODE_SHIFTED: [u8; 104] = [
    0, 0, 0, 0, b'A', b'B', b'C', b'D',
    b'E', b'F', b'G', b'H', b'I', b'J', b'K', b'L',
    b'M', b'N', b'O', b'P', b'Q', b'R', b'S', b'T',
    b'U', b'V', b'W', b'X', b'Y', b'Z', b'!', b'@',
    b'#', b'$', b'%', b'^', b'&', b'*', b'(', b')',
    b'\n', 0x1b, 0x7f, b'\t', b' ', b'_', b'+', b'{',
    b'}', b'|', b'~', b':', b'"', b'~', b'<', b'>',
    b'?', 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, b'/', b'*', b'-', b'+',
    b'\n', b'1', b'2', b'3', b'4', b'5', b'6', b'7',
    b'8', b'9', b'0', b'.', b'\\', 0, 0, b'=',
];

pub fn keyboard_handler(_modifire: u8, pressing: [u8; 6]) {
    keyboard_handler_internal(_modifire, pressing);
}

static mut MODSTATE: u8 = 0;
static mut STATE: [u8; 6] = [0; 6];

// 7個以上同時押しされた際の挙動は未定義
fn keyboard_handler_internal(modifire: u8, pressing: [u8; 6]) -> Option<()> {
    let shifted = modifire & (LSHIFT | RSHIFT) != 0;
    unsafe {
        if MODSTATE == modifire && STATE == pressing {
            let last = pressing.iter().rev().find(|v| **v != 0)?;
            let v = if !shifted {
                KEYCODE.get(*last as usize)?
            } else {
                KEYCODE_SHIFTED.get(*last as usize)?
            };
            if *v != 0 {
                print!("{}", (*v) as char);
            }
            Some(())
        } else {
            let pressing_len = pressing.iter().filter(|&&x| x != 0).count();
            let state_len = (*&raw const STATE).iter().filter(|&&x| x != 0).count();
            if pressing_len > state_len {
                let last = pressing.iter().rev().find(|&&v| v != 0)?;
                let v = if !shifted {
                    KEYCODE.get(*last as usize)?
                } else {
                    KEYCODE_SHIFTED.get(*last as usize)?
                };
                if *v != 0 {
                    print!("{}", (*v) as char);
                }
            }
            MODSTATE = modifire;
            STATE = pressing;
            None
        }
    }
}
