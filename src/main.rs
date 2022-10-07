// cargo rustc --release --target i686-pc-windows-msvc  -Zbuild-std=std,panic_abort -Zbuild-std-features=panic_immediate_abort -- -Ccontrol-flow-guard=off

#![no_main]
#![no_std]

#![windows_subsystem = "console"]

#![feature(start)]
#![feature(inline_const)]

use core::hint::unreachable_unchecked;
use core::ptr::null_mut;
use core::panic::PanicInfo;

use crate::KeyAction::{Down, Empty, Enter, Flag, Left, Right, Up};

// beginner: 9x9 w/ 10 @ 12.3%
// intermediate: 16x16 w/ 40 @ 15.6%
// expert: 30x16 w/ 99 @ 20.6%
const WIDTH: usize = 16;
const HEIGHT: usize = 16;
const MINE_COUNT: usize = 40;

const EMPTY_TYPE: u8 = 0b0000;
const WARNING_TYPE: u8 = 0b0100;
const MINE_TYPE: u8 = 0b1000;

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

#[start]
#[no_mangle]
fn main(_: isize, _: *const *const u8) -> isize {
    let mut board = [[0;WIDTH];HEIGHT];
    let mut start = 0;
    let mut non_mines_left = const { (WIDTH * HEIGHT) - MINE_COUNT };
    let mut x = 0;
    let mut y = 0;
    rerender_board(&board);
    set_cursor(2, 0);

    loop {
        match read_key() {
            Empty => {}
            Up => if y != 0 { // up key
                y -= 1;
            }
            Down => if y + 1 < HEIGHT { // down key
                y += 1;
            }
            Left => if x != 0 { // left key
                x -= 1;
            }
            Right => if x + 1 < WIDTH { // right key
                x += 1;
            }
            Flag => if board[y][x] & 1 == 0 { // f key
                board[y][x] ^= 0b10;
                print_tile(board[y][x]);
            }
            Enter => if board[y][x] & 0b11 == 0 { // enter key
                if start == 0 {
                    place_mines(&mut board, x, y);
                    start = unsafe { GetTickCount64() } as u64;
                }

                match board[y][x] & 0b1100 {
                    EMPTY_TYPE => {
                        let mut arr = [(x, y); WIDTH * HEIGHT];
                        let mut index = 0;
                        loop {
                            let (x, y) = arr[index];
                            if board[y][x] & 1 == 0 {
                                board[y][x] |= 1;
                                non_mines_left -= 1;
                                for &(x, y) in [(x - 1, y - 1), (x, y - 1), (x + 1, y - 1), (x - 1, y), (x + 1, y), (x - 1, y + 1), (x, y + 1), (x + 1, y + 1)].iter().filter(|(x, y)| x < &WIDTH && y < &HEIGHT) {
                                    if board[y][x] & 1 == 0 {
                                        if board[y][x] & 0b1100 == EMPTY_TYPE {
                                            arr[index] = (x, y);
                                            index += 1;
                                        } else {
                                            board[y][x] |= 1;
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

                        rerender_board(&board);
                    },
                    WARNING_TYPE => {
                        board[y][x] |= 1;
                        non_mines_left -= 1;
                        print_tile(board[y][x]);
                    },
                    _ => end(b"Game Over, you clicked a mine!", start)
                }
                if non_mines_left == 0 {
                    end(b"You win!", start);
                }
            }
        }
        set_cursor(x as i16 * 2 + 2, y as i16);
    }
}

#[inline(never)]
fn end(str: &[u8], start: u64) {
    set_cursor(0, HEIGHT as i16 + 1);
    print(str);
    print(b"\nPlaytime: ");
    print_usize(unsafe { (GetTickCount64() - start) / 1000 } as usize);
    print(b"s\n");
    unsafe { unreachable_unchecked() }
}

#[inline(never)]
fn stdout() -> *mut c_void {
    unsafe { GetStdHandle(const { -11i32 as u32 }) }
}

#[inline(never)]
fn stdout_bytes(ptr: *const u8, len: u32) {
    unsafe { WriteConsoleA(stdout(), ptr as *const _, len, null_mut(), null_mut()); };
}

#[inline(always)]
fn print(str: &[u8]) {
    stdout_bytes(str.as_ptr(), str.len() as u32);
}

#[inline(never)]
fn print_tile(tile: u8) {
    unsafe { WriteConsoleA(stdout(), {
        if tile & 0b10 == 0b10 {
            set_color(0b0100);
            b"$"
        } else if tile & 1 == 0 {
            b"_"
        } else {
            match tile & 0b1100 {
                EMPTY_TYPE => b" ",
                _ => match tile & 0b01110000 {
                    0b00000000 => {
                        set_color(0b1001);
                        b"1"
                    },
                    0b00010000 => {
                        set_color(0b0010);
                        b"2"
                    },
                    0b00100000 => {
                        set_color(0b1100);
                        b"3"
                    },
                    0b00110000 => {
                        set_color(0b0001);
                        b"4"
                    },
                    0b01000000 => {
                        set_color(0b0100);
                        b"5"
                    },
                    0b01010000 => {
                        set_color(0b0011);
                        b"6"
                    },
                    0b01100000 => {
                        set_color(0b1000);
                        b"7"
                    },
                    _ => {
                        set_color(0b1111);
                        b"8"
                    }
                }
            }
        }
    }.as_ptr() as *const _, 1, null_mut(), null_mut()); }
    set_color(0b0111);
}

#[inline(never)]
fn rerender_board(board: &[[u8;WIDTH];HEIGHT]) {
    set_cursor(0, 0);
    for i in 0..HEIGHT {
        print(b"[ ");
        for j in 0..WIDTH {
            print_tile(board[i][j]);
            print(b" ");
        }
        print(b"]\n")
    }
}

#[inline(never)]
fn set_color(color: u16) {
    unsafe { SetConsoleTextAttribute(stdout(), color); }
}

#[inline(always)]
fn place_mines(board: &mut [[u8;WIDTH];HEIGHT], input_x: usize, input_y: usize) {
    let mut random = Random::new();
    let mut i = 0;
    while i < MINE_COUNT {
        let x = random.usize() % WIDTH;
        let y = random.usize() % HEIGHT;
        if board[y][x] & 0b1100 != MINE_TYPE && !((y - input_y) + 1 <= 2 && (x - input_x) + 1 <= 2) {
            board[y][x] = MINE_TYPE;
            for &(x, y) in [(x - 1, y - 1), (x, y - 1), (x + 1, y - 1), (x - 1, y), (x + 1, y), (x - 1, y + 1), (x, y + 1), (x + 1, y + 1)].iter().filter(|(x, y)| x < &WIDTH && y < &HEIGHT) {
                match board[y][x] & 0b1100 {
                    EMPTY_TYPE => {
                        board[y][x] = WARNING_TYPE;
                    }
                    WARNING_TYPE => {
                        board[y][x] = (board[y][x] & 0b10001111) | ((board[y][x] & 0b01110000) + 16)
                    }
                    _ => {}
                }
            }
            i += 1;
        }
    }
}

#[inline(always)]
fn read_key() -> KeyAction {
    let first = unsafe { _getch() };
    if first == 13 {
        Enter
    } else if first == 224 {
        match unsafe { _getch() } {
            72 => Up,
            77 => Right,
            80 => Down,
            75 => Left,
            _ => Empty
        }
    } else if first == 102 {
        Flag
    } else {
        Empty
    }
}

#[inline(never)]
fn print_usize(mut usize: usize) {
    const MAX_LENGTH: usize = 10;
    let mut arr = [0; MAX_LENGTH];
    let mut offset = MAX_LENGTH - 1;
    while usize > 0 {
        arr[offset] = (usize % 10) as u8 + '0' as u8;
        offset -= 1;
        usize /= 10;
    }
    stdout_bytes(unsafe { arr.as_ptr().add(offset) }, (MAX_LENGTH - offset) as u32);
}

#[inline(never)]
fn set_cursor(x: i16, y: i16) {
    unsafe { SetConsoleCursorPosition(stdout(), (x, y)); }
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
struct Random(u64);

impl Random {
    #[inline(never)]
    fn usize(&mut self) -> usize {
        let before = self.0;
        let x = before ^ (before << 13);
        let x = x ^ (!x >> 17);
        self.0 = x ^ (x << 5);
        before as usize
    }

    #[inline(always)]
    fn new() -> Random {
        Random(unsafe { GetTickCount64() } as u64)
    }
}

#[panic_handler]
#[inline(always)]
fn panic_handler(_: &PanicInfo) -> ! {
    unsafe { unreachable_unchecked() }
}
