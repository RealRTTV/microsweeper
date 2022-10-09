// cargo rustc --release --target i686-pc-windows-msvc  -Zbuild-std=std,panic_abort -Zbuild-std-features=panic_immediate_abort -- -Ccontrol-flow-guard=off

#![no_builtins]
#![no_main]
#![no_std]

#![windows_subsystem = "console"]

#![feature(inline_const)]

use core::hint::unreachable_unchecked;
use core::panic::PanicInfo;
use crate::KeyAction::{Down, Empty, Enter, Flag, Left, Right, Up};

// beginner: 9x9 w/ 10 @ 12.3%
// intermediate: 16x16 w/ 40 @ 15.6%
// expert: 30x16 w/ 99 @ 20.6%
const WIDTH: usize = 30;
const HEIGHT: usize = 16;
const MINE_COUNT: usize = 99;

const EMPTY_TYPE: u8 = 0b0000;
const WARNING_TYPE: u8 = 0b01;
const MINE_TYPE: u8 = 0b10;

#[allow(non_camel_case_types)]
pub enum c_void {}

#[link(name = "msvcrt")]
extern {
    pub fn _getch() -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    #[allow(improper_ctypes)]
    pub fn SetConsoleCursorPosition(handle: *const c_void, pos: (i16, i16)) -> bool;

    pub fn SetConsoleTextAttribute(handle: *const c_void, attribs: u16) -> bool;

    pub fn GetTickCount64() -> u64;

    pub fn WriteConsoleA(handle: *const c_void, ptr: *const c_void, len: u32, num_chars_written: *mut u32, reserved: *mut c_void) -> bool;

    pub fn GetStdHandle(id: u32) -> *mut c_void;
}

#[no_mangle]
pub unsafe fn main() {
    let mut board = [[0;WIDTH];HEIGHT];
    let mut start = 0;
    let mut non_mines_left = const { (WIDTH * HEIGHT) - MINE_COUNT };
    let mut x = 0;
    let mut y = 0;
    rerender_board(&board, false);
    set_cursor(0, 0);

    loop {
        match read_key() {
            Empty => {}
            Up => if y > 0 { // up key
                y -= 1;
            }
            Down => if y + 1 < HEIGHT { // down key
                y += 1;
            }
            Left => if x > 0 { // left key
                x -= 1;
            }
            Right => if x + 1 < WIDTH { // right key
                x += 1;
            }
            Flag => if (*board.get_unchecked_mut(y).get_unchecked_mut(x)) & 0b100 == 0 { // f key
                (*board.get_unchecked_mut(y).get_unchecked_mut(x)) ^= 0b1000;
                print_tile(*board.get_unchecked_mut(y).get_unchecked_mut(x), false);
            }
            Enter => if (*board.get_unchecked_mut(y).get_unchecked_mut(x)) & 0b1100 == 0 { // enter key
                if start == 0 {
                    place_mines(&mut board, x, y);
                    start = GetTickCount64() as u32;
                }

                match (*board.get_unchecked_mut(y).get_unchecked_mut(x)) & 0b11 {
                    EMPTY_TYPE => {
                        let mut arr = [(x, y); WIDTH * HEIGHT];
                        let mut index = 0;
                        loop {
                            let (x, y) = arr[index];
                            let ptr = board.get_unchecked_mut(y).get_unchecked_mut(x);
                            if *ptr & 0b100 == 0 {
                                *ptr |= 0b100;
                                non_mines_left -= 1;
                                for &(x, y) in [(x - 1, y - 1), (x, y - 1), (x + 1, y - 1), (x - 1, y), (x + 1, y), (x - 1, y + 1), (x, y + 1), (x + 1, y + 1)].iter().filter(|(x, y)| x < &WIDTH && y < &HEIGHT) {
                                    let ptr = board.get_unchecked_mut(y).get_unchecked_mut(x);
                                    if *ptr & 0b100 == 0 {
                                        if *ptr & 0b11 == EMPTY_TYPE {
                                            arr[index] = (x, y);
                                            index += 1;
                                        } else {
                                            *ptr |= 0b100;
                                            non_mines_left -= 1;
                                        }
                                    }
                                }
                            }
                            index -= 1;
                            if index == usize::MAX {
                                break;
                            }
                        }

                        rerender_board(&board, false);
                    },
                    WARNING_TYPE => {
                        (*board.get_unchecked_mut(y).get_unchecked_mut(x)) |= 0b100;
                        non_mines_left -= 1;
                        print_tile(*board.get_unchecked_mut(y).get_unchecked_mut(x), false);
                    },
                    _ => end(b"Game Over!".as_ptr(), start, 10, &board),
                }
                if non_mines_left == 0 {
                    end(b"You win!".as_ptr(), start, 8, &board);
                }
            }
        }
        set_cursor(x, y);
    }
}

#[inline(never)]
unsafe fn end(str: *const u8, start: u32, str_len: u32, board: &[[u8;WIDTH];HEIGHT]) {
    set_cursor(!0, HEIGHT + 1);
    stdout_bytes(str, str_len);
    stdout_bytes(b"\nPlaytime: ".as_ptr(), 11);
    print_non_zero_usize(unsafe { (GetTickCount64() as u32 - start) / 1000 } as usize);
    stdout_bytes(b"s\n".as_ptr(), 2);
    rerender_board(board, true);
    unsafe { unreachable_unchecked() }
}

#[inline(never)]
unsafe fn stdout() -> *mut c_void {
    unsafe { GetStdHandle(const { -11i32 as u32 }) }
}

#[inline(never)]
unsafe fn stdout_bytes(ptr: *const u8, len: u32) {
    unsafe { WriteConsoleA(stdout(), ptr as *const _, len, 0 as *mut _, 0 as *mut _); };
}

#[inline(never)]
unsafe fn print_tile(tile: u8, mines: bool) {
    unsafe { WriteConsoleA(stdout(), &{
        if tile & 0b1000 > 0 {
            if mines {
                set_color(if tile & 0b11 == MINE_TYPE {
                    0b1011
                } else {
                    0b0101
                });
            } else {
                set_color(0b1110);
            }
            b'!'
        } else if tile & 0b100 == 0 {
            b'_'
        } else {
            match tile & 0b11 {
                EMPTY_TYPE => b' ',
                _ => {
                    set_color(match tile >> 4 {
                        0b0000 => 0b1001,
                        0b0001 => 0b0010,
                        0b0010 => 0b1100,
                        0b0011 => 0b0001,
                        0b0100 => 0b0100,
                        0b0101 => 0b0011,
                        0b0110 => 0b1000,
                        _ => 0b1111,
                    });
                    (tile >> 4) + b'1'
                }
            }
        }
    } as *const u8 as *const _, 1, 0 as *mut _, 0 as *mut _); }
    set_color(0b0111)
}

#[inline(never)]
unsafe fn rerender_board(board: &[[u8;WIDTH];HEIGHT], mines: bool) {
    set_cursor(!0, 0);
    for i in 0..HEIGHT {
        stdout_bytes(b"[ ".as_ptr(), 2);
        for j in 0..WIDTH {
            print_tile(*board.get_unchecked(i).get_unchecked(j), mines);
            stdout_bytes(b" ".as_ptr(), 1);
        }
        stdout_bytes(b"]\n".as_ptr(), 2)
    }
}

#[inline(never)]
unsafe fn set_color(color: u8) {
    unsafe { SetConsoleTextAttribute(stdout(), color as u16); }
}

#[inline(always)]
unsafe fn place_mines(board: &mut [[u8;WIDTH];HEIGHT], input_x: usize, input_y: usize) {
    let mut random = Random(GetTickCount64() as u32);
    let mut i = 0;
    while i < MINE_COUNT {
        let x = random.usize() % WIDTH;
        let y = random.usize() % HEIGHT;
        if !((y - input_y) <= 2 && (x - input_x) + 1 <= 2) && (*board.get_unchecked_mut(y).get_unchecked_mut(x)) & 0b11 != MINE_TYPE {
            (*board.get_unchecked_mut(y).get_unchecked_mut(x)) = MINE_TYPE;
            i += 1;
            for &(x, y) in [(x - 1, y - 1), (x, y - 1), (x + 1, y - 1), (x - 1, y), (x + 1, y), (x - 1, y + 1), (x, y + 1), (x + 1, y + 1)].iter().filter(|(x, y)| x < &WIDTH && y < &HEIGHT) {
                (*board.get_unchecked_mut(y).get_unchecked_mut(x)) = match (*board.get_unchecked_mut(y).get_unchecked_mut(x)) & 0b11 {
                    EMPTY_TYPE => {
                        WARNING_TYPE
                    }
                    WARNING_TYPE => {
                        ((*board.get_unchecked_mut(y).get_unchecked_mut(x)) & 0b00001111) | (((*board.get_unchecked_mut(y).get_unchecked_mut(x)) & 0b11110000) + 16)
                    }
                    _ => {
                        MINE_TYPE
                    }
                }
            }
        }
    }
}

#[inline(always)]
unsafe fn read_key() -> KeyAction {
    let first = _getch();
    if first == 13 {
        Enter
    } else if first == 102 {
        Flag
    } else if first == 224 {
        match _getch() {
            72 => Up,
            77 => Right,
            80 => Down,
            75 => Left,
            _ => Empty
        }
    } else {
        Empty
    }
}

#[inline(always)]
unsafe fn print_non_zero_usize(mut usize: usize) {
    while usize > 0 {
        stdout_bytes(&((usize % 10) as u8 + '0' as u8) as *const _, 1);
        usize /= 10;
    }
}

#[inline(never)]
unsafe fn set_cursor(x: usize, y: usize) {
    unsafe { SetConsoleCursorPosition(stdout(), (x as i16 * 2 + 2, y as i16)); }
}

#[repr(u8)]
enum KeyAction {
    Empty,
    Left,
    Right,
    Up,
    Down,
    Flag,
    Enter
}

#[repr(transparent)]
struct Random(u32);

impl Random {
    #[inline(never)]
    unsafe fn usize(&mut self) -> usize {
        let before = self.0;
        let x = before ^ (before << 13);
        let x = x ^ (!x >> 17);
        self.0 = x ^ (x << 5);
        before as usize
    }
}

#[inline(always)]
#[panic_handler]
pub unsafe fn panic(_: &PanicInfo) -> ! {
    unreachable_unchecked()
}
