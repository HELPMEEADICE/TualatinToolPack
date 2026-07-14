use std::{
    mem::zeroed,
    ptr::{null, null_mut},
};

use windows_sys::Win32::{
    Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{
        BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, HGDIOBJ, InvalidateRect,
        PAINTSTRUCT, UpdateWindow,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Input::KeyboardAndMouse::SetFocus,
        WindowsAndMessaging::{
            CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow,
            GWLP_USERDATA, GetClientRect, GetSystemMetrics, GetWindowLongPtrW, IDC_ARROW,
            LoadCursorW, RegisterClassW, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
            SM_YVIRTUALSCREEN, SW_SHOW, SetCursor, SetForegroundWindow, SetWindowLongPtrW,
            ShowWindow, WM_KEYDOWN, WM_LBUTTONDOWN, WM_NCCREATE, WM_NCDESTROY, WM_PAINT,
            WM_RBUTTONDOWN, WM_SETCURSOR, WNDCLASSW, WS_EX_APPWINDOW, WS_EX_TOPMOST, WS_POPUP,
            WS_VISIBLE,
        },
    },
};

const CLASS_NAME: &str = "TbToolRustDisplayTestWindow";
const VK_ESCAPE: usize = 0x1b;
const VK_SPACE: usize = 0x20;
const VK_LEFT: usize = 0x25;
const VK_RIGHT: usize = 0x27;
const COLORS: [COLORREF; 6] = [
    rgb(0, 0, 0),
    rgb(255, 255, 255),
    rgb(255, 0, 0),
    rgb(0, 255, 0),
    rgb(0, 0, 255),
    rgb(128, 128, 128),
];

struct State {
    color_index: usize,
}

pub unsafe fn show(owner: HWND) -> Result<(), std::io::Error> {
    let instance = unsafe { GetModuleHandleW(null()) };
    let class_name = wide(CLASS_NAME);
    let class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(window_proc),
        hInstance: instance,
        hCursor: unsafe { LoadCursorW(null_mut(), IDC_ARROW) },
        hbrBackground: null_mut(),
        lpszClassName: class_name.as_ptr(),
        ..unsafe { zeroed() }
    };
    unsafe { RegisterClassW(&class) };

    let state = Box::into_raw(Box::new(State { color_index: 0 }));
    let empty = wide("");
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_APPWINDOW | WS_EX_TOPMOST,
            class_name.as_ptr(),
            empty.as_ptr(),
            WS_POPUP | WS_VISIBLE,
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
            owner,
            null_mut(),
            instance,
            state.cast(),
        )
    };
    if hwnd.is_null() {
        unsafe { drop(Box::from_raw(state)) };
        return Err(std::io::Error::last_os_error());
    }
    unsafe {
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
        SetFocus(hwnd);
        UpdateWindow(hwnd);
    }
    Ok(())
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        let create = lparam as *const CREATESTRUCTW;
        let state = unsafe { (*create).lpCreateParams as *mut State };
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize) };
    }
    let state = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State };
    match message {
        WM_LBUTTONDOWN => {
            if !state.is_null() {
                unsafe { (*state).color_index = ((*state).color_index + 1) % COLORS.len() };
                unsafe { InvalidateRect(hwnd, null(), 1) };
            }
            0
        }
        WM_RBUTTONDOWN => {
            if !state.is_null() {
                unsafe {
                    (*state).color_index = ((*state).color_index + COLORS.len() - 1) % COLORS.len()
                };
                unsafe { InvalidateRect(hwnd, null(), 1) };
            }
            0
        }
        WM_KEYDOWN => {
            match wparam {
                VK_ESCAPE => unsafe {
                    DestroyWindow(hwnd);
                },
                VK_SPACE | VK_RIGHT => advance(state, hwnd, 1),
                VK_LEFT => advance(state, hwnd, COLORS.len() - 1),
                value @ 0x31..=0x36 if !state.is_null() => {
                    unsafe { (*state).color_index = value - 0x31 };
                    unsafe { InvalidateRect(hwnd, null(), 1) };
                }
                _ => {}
            }
            0
        }
        WM_SETCURSOR => {
            unsafe { SetCursor(null_mut()) };
            1
        }
        WM_PAINT => {
            let mut paint: PAINTSTRUCT = unsafe { zeroed() };
            let dc = unsafe { BeginPaint(hwnd, &mut paint) };
            let color = if state.is_null() {
                COLORS[0]
            } else {
                COLORS[unsafe { (*state).color_index }]
            };
            let brush = unsafe { CreateSolidBrush(color) };
            let mut rect: RECT = unsafe { zeroed() };
            unsafe {
                GetClientRect(hwnd, &mut rect);
                FillRect(dc, &rect, brush);
                DeleteObject(brush as HGDIOBJ);
                EndPaint(hwnd, &paint);
            }
            0
        }
        WM_NCDESTROY => {
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };
            if !state.is_null() {
                unsafe { drop(Box::from_raw(state)) };
            }
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn advance(state: *mut State, hwnd: HWND, amount: usize) {
    if state.is_null() {
        return;
    }
    unsafe {
        (*state).color_index = ((*state).color_index + amount) % COLORS.len();
        InvalidateRect(hwnd, null(), 1);
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

const fn rgb(red: u8, green: u8, blue: u8) -> COLORREF {
    red as u32 | ((green as u32) << 8) | ((blue as u32) << 16)
}
