//! The Win32 window the island lives in.
//!
//! This window is created here, by the process, with the styles the island needs:
//! `WS_POPUP` and without `WS_CAPTION` / `WS_EX_WINDOWEDGE`. That matters because a
//! tiling window manager such as komorebi only claims a window while it carries that
//! pair, and it decides when the window is *shown*, so a style fixed afterwards is
//! too late. This window is born without it.
//!
//! `WS_EX_LAYERED` gives the island true per-pixel alpha through
//! `UpdateLayeredWindow`: the pixels Slint renders, premultiplied, are exactly what
//! Windows wants. Windows also uses that alpha for hit testing, so later on a click
//! outside the rounded pill will not count as a click on the island.
//!
//! Not handled yet: DPI changes while running, and input events. Both belong to the
//! work that gives the island interaction; integration 2 only draws it.

use std::rc::Rc;

use slint::platform::PlatformError;
use slint::platform::software_renderer::{LineBufferProvider, PremultipliedRgbaColor, TargetPixel};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC,
    CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetMonitorInfoW, HBITMAP, HDC, HGDIOBJ,
    MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint, SelectObject, ValidateRect,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForSystem, GetDpiForWindow,
    SetProcessDpiAwarenessContext,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, MSG, PM_REMOVE, PeekMessageW, PostMessageW,
    PostQuitMessage, RegisterClassW, SW_HIDE, SW_SHOW, ShowWindow, ULW_ALPHA, UpdateLayeredWindow,
    WM_APP, WM_CLOSE, WM_DESTROY, WM_PAINT, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

/// The island's geometry, in logical pixels. This mirrors `ui/island.slint`, which
/// stays the single place where the island is defined; it is only used to size the
/// native window before Slint draws anything.
const LOGICAL_WIDTH: f32 = 260.0;
const LOGICAL_HEIGHT: f32 = 44.0;

const WINDOW_CLASS: &str = "capri-bar-island-window";
const WINDOW_TITLE: &str = "capri-bar";
/// Wakes the event loop when Slint asks for a redraw, so an idle island can sleep
/// with no timer running and still repaint the instant anything changes.
const REDRAW_MESSAGE: u32 = WM_APP + 1;

/// The island window, its drawing surface and its position on screen.
pub struct IslandWindow {
    hwnd: HWND,
    memory_dc: HDC,
    bitmap: HBITMAP,
    pixels: *mut BgraPixel,
    width: usize,
    height: usize,
    scale_factor: f32,
}

impl IslandWindow {
    /// Creates the island window, hidden, with the styles described in the module
    /// documentation, and a top-down 32-bit surface for Slint to render into.
    pub fn create() -> Result<Rc<Self>, PlatformError> {
        // Without this the process would be DPI-virtualized and the island would be
        // sized and positioned in virtual pixels.
        unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };

        let scale_factor = (unsafe { GetDpiForSystem() }) as f32 / 96.0;
        let width = (LOGICAL_WIDTH * scale_factor).round().max(1.0) as usize;
        let height = (LOGICAL_HEIGHT * scale_factor).round().max(1.0) as usize;

        unsafe {
            let instance = GetModuleHandleW(std::ptr::null());
            let class_name = wide(WINDOW_CLASS);
            let window_title = wide(WINDOW_TITLE);

            let window_class = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: instance,
                lpszClassName: class_name.as_ptr(),
                ..std::mem::zeroed()
            };
            if RegisterClassW(&window_class) == 0 {
                return Err(PlatformError::Other(
                    "RegisterClassW failed for the island window".into(),
                ));
            }

            // The island never steals the focus, stays on top, keeps out of Alt+Tab,
            // and is a popup without a frame: exactly the pair komorebi needs is
            // absent, and it never gets added later.
            let style = WS_POPUP;
            let ex_style = WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE;

            let hwnd = CreateWindowExW(
                ex_style,
                class_name.as_ptr(),
                window_title.as_ptr(),
                style,
                0,
                0,
                width as i32,
                height as i32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            );
            if hwnd.is_null() {
                return Err(PlatformError::Other(
                    "CreateWindowExW failed for the island window".into(),
                ));
            }

            let scale_factor = GetDpiForWindow(hwnd) as f32 / 96.0;
            let width = (LOGICAL_WIDTH * scale_factor).round().max(1.0) as usize;
            let height = (LOGICAL_HEIGHT * scale_factor).round().max(1.0) as usize;

            // A top-down 32-bit DIB: the pixels are BGRA, premultiplied, which is
            // what UpdateLayeredWindow expects and what Slint's software renderer
            // writes through BgraPixel.
            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..std::mem::zeroed()
            };

            let memory_dc = CreateCompatibleDC(std::ptr::null_mut());
            let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
            let bitmap = CreateDIBSection(
                memory_dc,
                &info,
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            );
            if memory_dc.is_null() || bitmap.is_null() || bits.is_null() {
                return Err(PlatformError::Other(
                    "could not create the island drawing surface".into(),
                ));
            }
            SelectObject(memory_dc, bitmap as HGDIOBJ);

            let window = Rc::new(Self {
                hwnd,
                memory_dc,
                bitmap,
                pixels: bits as *mut BgraPixel,
                width,
                height,
                scale_factor,
            });
            window.centre_at_top_of_primary_monitor();
            Ok(window)
        }
    }

    /// Logical-to-physical factor of the monitor the island was created on.
    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    /// The size of the drawing surface in physical pixels, which is also the size of
    /// the window: the island does not grow by being resized.
    pub fn physical_size(&self) -> (u32, u32) {
        (self.width as u32, self.height as u32)
    }

    pub fn show(&self) {
        unsafe { ShowWindow(self.hwnd, SW_SHOW) };
    }

    pub fn hide(&self) {
        unsafe { ShowWindow(self.hwnd, SW_HIDE) };
    }

    /// Asks the event loop to look at Slint again.
    pub fn wake(&self) {
        unsafe { PostMessageW(self.hwnd, REDRAW_MESSAGE, 0, 0) };
    }

    /// Runs `draw` with a line-by-line view over the window pixels, which is how
    /// Slint renders into a surface of ours without any intermediate copy.
    pub fn with_line_buffer<R>(&self, draw: impl FnOnce(IslandLineBuffer<'_>) -> R) -> R {
        // SAFETY: the DIB is owned by this window and outlives it; the slice never
        // escapes the closure, and this whole module is used from a single thread.
        let pixels = unsafe { std::slice::from_raw_parts_mut(self.pixels, self.width * self.height) };
        draw(IslandLineBuffer {
            pixels,
            width: self.width,
        })
    }

    /// Pushes the rendered pixels to the screen, with their alpha.
    pub fn present(&self) -> Result<(), PlatformError> {
        let mut destination = POINT {
            x: self.window_rect().0,
            y: self.window_rect().1,
        };
        let size = SIZE {
            cx: self.width as i32,
            cy: self.height as i32,
        };
        let source = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };

        let updated = unsafe {
            UpdateLayeredWindow(
                self.hwnd,
                std::ptr::null_mut(),
                &mut destination,
                &size,
                self.memory_dc,
                &source,
                0,
                &blend,
                ULW_ALPHA,
            )
        };
        if updated == 0 {
            return Err(PlatformError::Other(
                "UpdateLayeredWindow failed to present the island".into(),
            ));
        }
        Ok(())
    }

    fn window_rect(&self) -> (i32, i32) {
        unsafe {
            let mut rect: windows_sys::Win32::Foundation::RECT = std::mem::zeroed();
            windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(self.hwnd, &mut rect);
            (rect.left, rect.top)
        }
    }

    /// Centred horizontally at the top of the primary monitor's work area.
    fn centre_at_top_of_primary_monitor(&self) {
        unsafe {
            let monitor = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
            let mut info: MONITORINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(monitor, &mut info) == 0 {
                return;
            }
            let work = info.rcWork;
            let x = work.left + ((work.right - work.left) - self.width as i32) / 2;
            let y = work.top;
            windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos(
                self.hwnd,
                std::ptr::null_mut(),
                x,
                y,
                self.width as i32,
                self.height as i32,
                windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE
                    | windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOZORDER,
            );
        }
    }
}

impl Drop for IslandWindow {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
            DeleteObject(self.bitmap as HGDIOBJ);
            DeleteDC(self.memory_dc);
        }
    }
}

/// A 32-bit BGRA pixel with premultiplied alpha: the memory layout of a top-down
/// Windows DIB, on little-endian, is exactly `0xAARRGGBB`.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct BgraPixel(u32);

impl TargetPixel for BgraPixel {
    fn blend(&mut self, color: PremultipliedRgbaColor) {
        let [blue, green, red, alpha] = self.0.to_le_bytes();
        let inverse = (u8::MAX - color.alpha) as u16;
        self.0 = u32::from_le_bytes([
            (blue as u16 * inverse / 255) as u8 + color.blue,
            (green as u16 * inverse / 255) as u8 + color.green,
            (red as u16 * inverse / 255) as u8 + color.red,
            (alpha as u16 + color.alpha as u16 - (alpha as u16 * color.alpha as u16) / 255) as u8,
        ]);
    }

    fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        Self(u32::from_le_bytes([blue, green, red, u8::MAX]))
    }

    fn background() -> Self {
        // The island's background is transparent, so a frame starts empty and only
        // what the island draws becomes opaque.
        Self(0)
    }
}

/// The drawing surface of the island, seen line by line by Slint's renderer.
pub struct IslandLineBuffer<'a> {
    pixels: &'a mut [BgraPixel],
    width: usize,
}

impl LineBufferProvider for IslandLineBuffer<'_> {
    type TargetPixel = BgraPixel;

    fn process_line(
        &mut self,
        line: usize,
        range: std::ops::Range<usize>,
        render_fn: impl FnOnce(&mut [Self::TargetPixel]),
    ) {
        let line_begin = line * self.width;
        render_fn(&mut self.pixels[line_begin..][range]);
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        // A layered window is painted by UpdateLayeredWindow, never by GDI.
        WM_PAINT => {
            unsafe { ValidateRect(hwnd, std::ptr::null()) };
            0
        }
        WM_CLOSE => {
            unsafe { DestroyWindow(hwnd) };
            0
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

/// Processes every Win32 message that is already queued. Returns false when the
/// application has been asked to quit.
pub fn process_pending_messages() -> bool {
    let mut message: MSG = unsafe { std::mem::zeroed() };
    loop {
        let pending = unsafe { PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) };
        if pending == 0 {
            return true;
        }
        if message.message == windows_sys::Win32::UI::WindowsAndMessaging::WM_QUIT {
            return false;
        }
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::TranslateMessage(&message);
            windows_sys::Win32::UI::WindowsAndMessaging::DispatchMessageW(&message);
        }
    }
}
