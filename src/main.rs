// cargo +nightly rustc --release --target x86_64-pc-windows-msvc  -Zbuild-std=std,panic_abort -Zbuild-std-features=panic_immediate_abort -- -Ccontrol-flow-guard=off

#![no_main]
#![no_std]

#![windows_subsystem = "console"]

#![feature(allocator_api)]
#![feature(alloc_error_handler)]
#![feature(start)]
#![feature(box_syntax)]
#![feature(inline_const)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::string::ToString;
use core::alloc::GlobalAlloc;
use core::alloc::Layout;
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
use winapi::um::heapapi::HeapAlloc;
use winapi::um::heapapi::GetProcessHeap;
use winapi::um::heapapi::HeapFree;

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
            0 => {}
            1 => if y != 0 { // up key
                y -= 1;
                print("\x1B[1A");
            }
            2 => if y + 1 < HEIGHT { // down key
                y += 1;
                print("\x1B[1B");
            }
            3 => if x != 0 { // left key
                x -= 1;
                print("\x1B[2D");
            }
            4 => if x + 1 < WIDTH { // right key
                x += 1;
                print("\x1B[2C");
            }
            5 => if board[y][x] & 1 == 0 { // f key
                board[y][x] ^= 0b10;
                print(fmt(board[y][x]));
                print("\x1B[1D");
            }
            6 => if board[y][x] & 0b11 == 0 { // enter key
                if start.is_none() {
                    place_mines(&mut board, x, y);
                    start = Some( unsafe { GetTickCount64() } as u64 );
                }

                match board[y][x] & 0b1100 {
                    EMPTY_TYPE => {
                        let mut wrapped = Some(box Node::new((x, y)));
                        while let Some(mut node) = wrapped {
                            let (x, y) = node.value;
                            if board[y][x] & 1 == 0 {
                                board[y][x] |= 1;
                                non_mines_left -= 1;
                                for &(x, y) in [(x - 1, y - 1), (x, y - 1), (x + 1, y - 1), (x - 1, y), (x + 1, y), (x - 1, y + 1), (x, y + 1), (x + 1, y + 1)].iter().filter(|(x, y)| x < &WIDTH && y < &HEIGHT) {
                                    if board[y][x] & 1 == 0 {
                                        if board[y][x] & 0b1100 == EMPTY_TYPE {
                                            let x = Some(box Node { value: (x, y), next: node.next });
                                            node.next = x;
                                        } else {
                                            board[y][x] |= 1;
                                            non_mines_left -= 1;
                                        }
                                    }
                                }
                            }
                            wrapped = node.next;
                        }

                        rerender_board(&board, x, y);
                    },
                    WARNING_TYPE => {
                        board[y][x] |= 1;
                        non_mines_left -= 1;
                        print(fmt(board[y][x]));
                        print("\x1B[1D");
                    },
                    MINE_TYPE => {
                        print("\x1B[");
                        print(&(HEIGHT - y).to_string());
                        print("B\x1B[");
                        print(&(x * 2 + 2).to_string());
                        print("D\n\x1B[37mGame Over, you clicked a mine!\nPlaytime: ");
                        print(&timestamp(start));
                        print("s\n");
                        loop {}
                    }
                    _ => unsafe { unreachable_unchecked() }
                }
                if non_mines_left == 0 {
                    print("\x1B[");
                    print(&(HEIGHT - y).to_string());
                    print("B\x1B[");
                    print(&(x * 2 + 2).to_string());
                    print("D\n\x1B[37mYou win!\nPlaytime: ");
                    print(&timestamp(start));
                    print("s\n");
                    loop {}
                }
            }
            _ => unsafe { unreachable_unchecked() }
        }
    }
}

#[inline(never)]
fn print(str: &str) {
    unsafe { WriteConsoleA(GetStdHandle(-11i32 as u32), str.as_ptr() as *const _, str.len() as u32, null_mut(), null_mut()); }
}

#[inline(never)]
fn timestamp(start: Option<u64>) -> String {
    start.map(|x| ((unsafe { GetTickCount64() } as u64 - x) as f64 / 1000.0).to_string()).unwrap_or("[Failed to get playtime]".to_string())
}

#[inline(never)]
fn fmt(tile: u8) -> &'static str {
    if tile & 0b10 == 0b10 {
        return "\x1B[36;1m$";
    }

    if tile & 1 == 0 {
        return "\x1B[37m_";
    }

    match tile & 0b1100 {
        EMPTY_TYPE => "\x1B[30m ",
        WARNING_TYPE => match tile & 0b01110000 {
            0b00000000 => "\x1B[34;1m1",
            0b00010000 => "\x1B[32m2",
            0b00100000 => "\x1B[31;1m3",
            0b00110000 => "\x1B[34m4",
            0b01000000 => "\x1B[31m5",
            0b01010000 => "\x1B[37m6",
            0b01100000 => "\x1B[35m7",
            0b01110000 => "\x1B[37m8",
            _ => unsafe { unreachable_unchecked() }
        },
        MINE_TYPE => "\x1B[31;0mX",
        _ => unsafe { unreachable_unchecked() }
    }
}

#[inline(never)]
fn rerender_board(board: &[[u8;WIDTH];HEIGHT], x: usize, y: usize) {
    let mut str = String::new();
    for i in 0..HEIGHT {
        str.push_str("[ ");
        for j in 0..WIDTH {
            str.push_str(fmt(board[i][j]));
            str.push(' ');
        }
        str.push_str("\x1B[37m]\n");
    }
    print("\x1Bc");
    print(&str);
    print("\x1B[");
    print(&(y + 1).to_string());
    print(";");
    print(&(x * 2 + 3).to_string());
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
unsafe fn read_key() -> u8 {
    let mut buffer: INPUT_RECORD = zeroed();
    ReadConsoleInputA(GetStdHandle(-10i32 as u32), &mut buffer, 1, &mut zeroed());
    if buffer.EventType == KEY_EVENT {
        let key_event: KEY_EVENT_RECORD = transmute(buffer.Event);
        if key_event.bKeyDown == 0 {
            0
        } else {
            let char = *key_event.uChar.AsciiChar();
            if char == 'f' as i8 {
                5
            } else if char == 0 {
                match key_event.wVirtualKeyCode as i32 {
                    0x26 => 1,
                    0x28 => 2,
                    0x25 => 3,
                    0x27 => 4,
                    0x0D => 6,
                    _ => 0
                }
            } else if char == '\r' as i8 {
                6
            } else {
                0
            }
        }
    } else {
        0
    }
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

struct Node<T> {
    value: T,
    next: Option<Box<Node<T>>>
}

impl<T> Node<T> {
    #[inline(always)]
    fn new(value: T) -> Node<T> {
        Node { value, next: None }
    }
}

#[panic_handler]
#[inline(always)]
fn panic_handler(_: &PanicInfo) -> ! {
    unsafe { unreachable_unchecked() }
}

#[inline(always)]
#[alloc_error_handler]
fn alloc_handler(_: Layout) -> ! {
    unsafe { unreachable_unchecked() }
}

#[global_allocator]
static GLOBAL: MyAllocator = MyAllocator;

struct MyAllocator;

unsafe impl GlobalAlloc for MyAllocator {
    #[inline(never)]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        HeapAlloc(GetProcessHeap(), 0, layout.size()) as *mut u8
    }

    #[inline(never)]
    unsafe fn dealloc(&self, ptr: *mut u8, _: Layout) {
        HeapFree(GetProcessHeap(), 0, ptr as *mut _);
    }
}
