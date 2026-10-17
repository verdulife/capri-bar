// Spike (throwaway): can a window be *born* ineligible for a tiling window manager?
//
// komorebi 0.1.41 decides in `window_is_eligible` (komorebi/src/window.rs) that a
// window must carry `WS_CAPTION` and `WS_EX_WINDOWEDGE` together to be managed, and
// winit -- which creates Slint's window -- sets both unconditionally for top-level
// windows. So the question this spike answers is whether a window created by hand
// with `WS_POPUP` and neither of those bits is really ignored from birth.
//
// Nothing here is patched after creation: the styles printed below are the ones the
// window was created with, read back from Windows itself.

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BLACK_BRUSH, BeginPaint, EndPaint, FillRect, GetStockObject, PAINTSTRUCT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{GetCurrentProcessId, Sleep};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GWL_EXSTYLE, GWL_STYLE, GetWindowLongW,
    GetWindowRect, IsWindowVisible, MSG, PM_REMOVE, PeekMessageW, PostQuitMessage, RegisterClassW,
    SW_SHOW, ShowWindow, TranslateMessage, WM_DESTROY, WM_ERASEBKGND, WM_PAINT, WM_QUIT,
    WNDCLASSW, WS_CAPTION, WS_EX_APPWINDOW, WS_EX_TOPMOST, WS_EX_WINDOWEDGE, WS_POPUP, WS_VISIBLE,
};

const CLASS_NAME: &str = "capri-bar-spike-window";
const ISLAND_WIDTH: i32 = 260;
const ISLAND_HEIGHT: i32 = 44;

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            // Paint the client area black so there is something to look at.
            let mut paint: PAINTSTRUCT = unsafe { std::mem::zeroed() };
            let hdc = unsafe { BeginPaint(hwnd, &mut paint) };
            unsafe { FillRect(hdc, &paint.rcPaint, GetStockObject(BLACK_BRUSH)) };
            unsafe { EndPaint(hwnd, &paint) };
            0
        }
        WM_ERASEBKGND => 1,
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn main() {
    let seconds: u64 = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(20);

    let pid = unsafe { GetCurrentProcessId() };
    println!("win32-spike: pid {pid}, vivira {seconds} s");

    // The two styles komorebi looks for: this window must never acquire them.
    let requested_style = WS_POPUP | WS_VISIBLE;
    let requested_ex_style = WS_EX_TOPMOST;

    let (hwnd, born_ineligible) = unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let class_name = wide(CLASS_NAME);
        let window_title = wide("capri-bar-spike");

        let window_class = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance,
            lpszClassName: class_name.as_ptr(),
            ..std::mem::zeroed()
        };
        if RegisterClassW(&window_class) == 0 {
            eprintln!("RegisterClassW fallo");
            std::process::exit(2);
        }

        let hwnd = CreateWindowExW(
            requested_ex_style,
            class_name.as_ptr(),
            window_title.as_ptr(),
            requested_style,
            100,
            100,
            ISLAND_WIDTH,
            ISLAND_HEIGHT,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        );
        if hwnd.is_null() {
            eprintln!("CreateWindowExW fallo");
            std::process::exit(2);
        }

        let got_style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        let got_ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;

        println!("estilo  pedido a CreateWindowExW: 0x{requested_style:08X}");
        println!("exestilo pedido a CreateWindowExW: 0x{requested_ex_style:08X}");
        println!();
        println!("estilo  leido de Windows: 0x{got_style:08X}");
        println!(
            "  WS_CAPTION   = {}   WS_POPUP = {}   WS_VISIBLE = {}",
            bit(got_style, WS_CAPTION),
            bit(got_style, WS_POPUP),
            bit(got_style, WS_VISIBLE)
        );
        println!("exestilo leido de Windows: 0x{got_ex_style:08X}");
        println!(
            "  WS_EX_WINDOWEDGE = {}   WS_EX_TOPMOST = {}   WS_EX_APPWINDOW = {}",
            bit(got_ex_style, WS_EX_WINDOWEDGE),
            bit(got_ex_style, WS_EX_TOPMOST),
            bit(got_ex_style, WS_EX_APPWINDOW)
        );

        let born_ineligible =
            (got_style & WS_CAPTION) == 0 && (got_ex_style & WS_EX_WINDOWEDGE) == 0;
        println!();
        if born_ineligible {
            println!(
                "VEREDICTO: la ventana nace SIN WS_CAPTION ni WS_EX_WINDOWEDGE -> inelegible para komorebi"
            );
        } else {
            println!(
                "VEREDICTO: la ventana lleva el par que komorebi exige -> ELEGIBLE (spike fallido)"
            );
        }

        ShowWindow(hwnd, SW_SHOW);
        let mut rect: RECT = std::mem::zeroed();
        GetWindowRect(hwnd, &mut rect);
        println!(
            "ventana: {}x{} en L{} T{}, visible={}",
            rect.right - rect.left,
            rect.bottom - rect.top,
            rect.left,
            rect.top,
            IsWindowVisible(hwnd) != 0
        );
        println!("(compruebame con: komorebic state -> capri-bar-spike.exe no debe aparecer)");

        (hwnd, born_ineligible)
    };

    // Message loop with a deadline, so the spike closes itself.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    while std::time::Instant::now() < deadline {
        while unsafe { PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
            if msg.message == WM_QUIT {
                std::process::exit(if born_ineligible { 0 } else { 1 });
            }
            unsafe { TranslateMessage(&msg) };
            unsafe { DispatchMessageW(&msg) };
        }
        unsafe { Sleep(20) };
    }

    println!("win32-spike: fin, cierro");
    unsafe { windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd) };
    std::process::exit(if born_ineligible { 0 } else { 1 });
}

/// Prints SI/no for one style bit, so the console output reads like a checklist.
fn bit(value: u32, flag: u32) -> &'static str {
    if value & flag == flag { "SI" } else { "no" }
}
