use crate::print;

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

pub fn keyboard_handler(_modifire: u8, pressing: [u8; 6]) {
    keyboard_handler_internal(_modifire, pressing);
}

static mut STATE: [u8; 6] = [0; 6];

// 7個以上同時押しされた際の挙動は未定義
fn keyboard_handler_internal(_modifire: u8, pressing: [u8; 6]) -> Option<()> {
    unsafe {
        if STATE == pressing {
            let last = pressing.iter().rev().find(|v| **v != 0)?;
            let v = KEYCODE.get(*last as usize)?;
            if *v != 0 {
                print!("{}", (*v) as char);
            }
            Some(())
        } else {
            let pressing_len = pressing.iter().filter(|&&x| x != 0).count();
            let state_len = (*&raw const STATE).iter().filter(|&&x| x != 0).count();
            if pressing_len > state_len {
                let last = pressing.iter().rev().find(|&&v| v != 0)?;
                let v = KEYCODE.get(*last as usize)?;
                if *v != 0 {
                    print!("{}", (*v) as char);
                }
            }
            STATE = pressing;
            None
        }
    }
}
