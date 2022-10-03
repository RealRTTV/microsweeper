// cargo +nightly rustc --release --target x86_64-pc-windows-msvc  -Zbuild-std=std,panic_abort -Zbuild-std-features=panic_immediate_abort -- -Ccontrol-flow-guard=off

#![no_main]
#![no_std]

#![windows_subsystem = "console"]

#![feature(allocator_api)]
#![feature(alloc_error_handler)]
#![feature(start)]
#![feature(box_syntax)]
#![feature(inline_const)]
#![feature(int_log)]

use core::hint::unreachable_unchecked;
use core::ptr::null_mut;
use core::mem::transmute;
use core::mem::zeroed;
use core::panic::PanicInfo;

use winapi::um::consoleapi::ReadConsoleInputA;
use winapi::um::consoleapi::WriteConsoleA;
use winapi::um::processenv::GetStdHandle;
use winapi::um::wincontypes::KEY_EVENT_RECORD;
use winapi::um::wincontypes::INPUT_RECORD;
use winapi::um::wincontypes::KEY_EVENT;
use winapi::um::sysinfoapi::GetTickCount64;
use crate::KeyAction::{Down, Empty, Enter, Flag, Left, Right, Up};

// beginner: 9x9 w/ 10 @ 12.3%
// intermediate: 16x16 w/ 40 @ 15.6%
// expert: 30x16 w/ 99 @ 20.6%
const WIDTH: usize = 9;
const HEIGHT: usize = 9;
const MINE_COUNT: usize = 10;

const EMPTY_TYPE: u8 = 0b0000;
const WARNING_TYPE: u8 = 0b0100;
const MINE_TYPE: u8 = 0b1000;

#[link(name = "msvcrt")]
extern {}

#[start]
#[no_mangle]
fn main(_: isize, _: *const *const u8) -> isize {
    let mut board = [[0;WIDTH];HEIGHT];
    let mut start = None;
    let mut non_mines_left = const { (WIDTH * HEIGHT) - MINE_COUNT };
    let mut x = 0;
    let mut y = 0;
    rerender_board(&board, x, y);

    loop {
        match unsafe { read_key() } {
            Empty => {}
            Up => if y != 0 { // up key
                y -= 1;
                print("\x1B[1A");
            }
            Down => if y + 1 < HEIGHT { // down key
                y += 1;
                print("\x1B[1B");
            }
            Left => if x != 0 { // left key
                x -= 1;
                print("\x1B[2D");
            }
            Right => if x + 1 < WIDTH { // right key
                x += 1;
                print("\x1B[2C");
            }
            Flag => if board[y][x] & 1 == 0 { // f key
                board[y][x] ^= 0b10;
                stdout_bytes(fmt(board[y][x]).as_ptr(), 8);
                print("\x1B[1D");
            }
            Enter => if board[y][x] & 0b11 == 0 { // enter key
                if start.is_none() {
                    place_mines(&mut board, x, y);
                    start = Some( unsafe { GetTickCount64() } as u64 );
                }

                match board[y][x] & 0b1100 {
                    EMPTY_TYPE => {
                        let mut arr = [(0, 0); WIDTH * HEIGHT];
                        arr[0] = (x, y);
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

                        rerender_board(&board, x, y);
                    },
                    WARNING_TYPE => {
                        board[y][x] |= 1;
                        non_mines_left -= 1;
                        unsafe { WriteConsoleA(GetStdHandle(-11i32 as u32), fmt(board[y][x]).as_ptr() as *const _, 8_u32, null_mut(), null_mut()); };
                        print("\x1B[1D");
                    },
                    MINE_TYPE => {
                        print("\x1B[");
                        print_usize(HEIGHT - y);
                        print("B\x1B[");
                        print_usize(x * 2 + 2);
                        print("D\n\x1B[37mGame Over, you clicked a mine!\nPlaytime: ");
                        timestamp(unsafe { start.unwrap_unchecked() });
                        print("s\n");
                        loop {}
                    }
                    _ => unsafe { unreachable_unchecked() }
                }
                if non_mines_left == 0 {
                    print("\x1B[");
                    print_usize(HEIGHT - y);
                    print("B\x1B[");
                    print_usize(x * 2 + 2);
                    print("D\n\x1B[37mYou win!\nPlaytime: ");
                    timestamp(unsafe { start.unwrap_unchecked() });
                    print("s\n");
                    loop {}
                }
            }
        }
    }
}

#[inline(never)]
fn stdout_bytes(ptr: *const u8, len: u32) {
    unsafe { WriteConsoleA(GetStdHandle(-11i32 as u32), ptr as *const _, len, null_mut(), null_mut()); };
}

#[inline(always)]
fn print(str: &'static str) {
    stdout_bytes(str.as_ptr(), str.len() as u32);
}

#[inline(never)]
fn timestamp(start: u64) {
    print_usize(unsafe { (GetTickCount64() as u64 - start) / 1000 } as usize)
}

#[inline(never)]
fn fmt(tile: u8) -> &'static [u8; 8] {
    if tile & 0b10 == 0b10 {
        return b"\x1B[36;1m$";
    }

    // if tile & 0b1100 == MINE_TYPE {
    //     return b"\x1B[31\0\0mX";
    // }

    if tile & 1 == 0 {
        return b"\x1B[37m\0\0_";
    }

    match tile & 0b1100 {
        EMPTY_TYPE => b"\x1B[30m\0\0 ",
        WARNING_TYPE => match tile & 0b01110000 {
            0b00000000 => b"\x1B[34;1m1",
            0b00010000 => b"\x1B[32m\0\02",
            0b00100000 => b"\x1B[31;1m3",
            0b00110000 => b"\x1B[34m\0\04",
            0b01000000 => b"\x1B[31m\0\05",
            0b01010000 => b"\x1B[37m\0\06",
            0b01100000 => b"\x1B[35m\0\07",
            0b01110000 => b"\x1B[37m\0\08",
            _ => unsafe { unreachable_unchecked() }
        },
        MINE_TYPE => b"\x1B[31\0\0mX",
        _ => unsafe { unreachable_unchecked() }
    }
}

#[inline(never)]
fn rerender_board(board: &[[u8;WIDTH];HEIGHT], x: usize, y: usize) {
    let mut array = [b' '; HEIGHT * (WIDTH * 9 + 9)];
    for i in 0..HEIGHT {
        array[i * (WIDTH * 9 + 9)] = b'[';
        for j in 0..WIDTH {
            array[(i * (WIDTH * 9 + 9) + (j * 9 + 2))..][..8].copy_from_slice(fmt(board[i][j]));
        }
        array[(i * (WIDTH * 9 + 9) + (WIDTH * 9 + 2))..][..7].copy_from_slice(b"\x1B[37m]\n");
    }
    print("\x1Bc");
    stdout_bytes(array.as_ptr(), (HEIGHT * (WIDTH * 9 + 9)) as u32);
    print("\x1B[");
    print_usize(y + 1);
    print(";");
    print_usize(x * 2 + 3);
    print("f");
}

#[inline(always)]
fn place_mines(board: &mut [[u8;WIDTH];HEIGHT], input_x: usize, input_y: usize) {
    let mut random = Random::new();
    let mut i = 0;
    while i < MINE_COUNT {
        let x = random.usize() % WIDTH;
        let y = random.usize() % HEIGHT;
        if board[y][x] & 0b1100 != MINE_TYPE && !(x == input_x && (y == input_y || y + 1 == input_y || y == input_y + 1)) && !(x == input_x + 1 && (y == input_y || y + 1 == input_y || y == input_y + 1)) && !(x + 1 == input_x && (y == input_y || y + 1 == input_y || y == input_y + 1)) {
            board[y][x] = MINE_TYPE;
            for &(x, y) in [(x - 1, y - 1), (x, y - 1), (x + 1, y - 1), (x - 1, y), (x + 1, y), (x - 1, y + 1), (x, y + 1), (x + 1, y + 1)].iter().filter(|(x, y)| x < &WIDTH && y < &HEIGHT) {
                match board[y][x] & 0b1100 {
                    EMPTY_TYPE => {
                        board[y][x] = WARNING_TYPE;
                    }
                    WARNING_TYPE => {
                        board[y][x] = (board[y][x] & 0b10001111) | ((board[y][x] & 0b01110000) + 16)
                    }
                    MINE_TYPE => {}
                    _ => unsafe { unreachable_unchecked() }
                }
            }
            i += 1;
        }
    }
}

#[inline(always)]
unsafe fn read_key() -> KeyAction {
    let mut buffer: INPUT_RECORD = zeroed();
    ReadConsoleInputA(GetStdHandle(-10i32 as u32), &mut buffer, 1, &mut zeroed());
    if buffer.EventType == KEY_EVENT {
        let key_event: KEY_EVENT_RECORD = transmute(buffer.Event);
        if key_event.bKeyDown == 0 {
            Empty
        } else {
            let char = *key_event.uChar.AsciiChar();
            if char == 'f' as i8 {
                Flag
            } else if char == 0 {
                match key_event.wVirtualKeyCode as i32 {
                    0x26 => Up,
                    0x28 => Down,
                    0x25 => Left,
                    0x27 => Right,
                    0x0D => Enter,
                    _ => Empty
                }
            } else if char == '\r' as i8 {
                Enter
            } else {
                Empty
            }
        }
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
        let x = x ^ (x >> 17);
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
