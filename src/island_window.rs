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
//! The size is not decided here: `ui/island.slint` is the only place where the island
//! is defined, so the surface follows the size Slint asks for (see
//! `IslandWindow::resize_surface` and its caller, `WindowAdapter::update_window_properties`).
//!
//! Not handled yet: input events, and changes of the monitor work area. Both belong to
//! the work that gives the island interaction; integration 2 only draws it.

use std::cell::RefCell;
use std::rc::Rc;

use slint::platform::PlatformError;
use slint::platform::software_renderer::{LineBufferProvider, PremultipliedRgbaColor, TargetPixel};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetMonitorInfoW,
    HBITMAP, HDC, HGDIOBJ, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint, SelectObject,
    ValidateRect,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow, SetProcessDpiAwarenessContext,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, MSG, PM_REMOVE, PeekMessageW, PostMessageW,
    PostQuitMessage, RegisterClassW, SW_HIDE, SW_SHOW, SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos,
    ShowWindow, ULW_ALPHA, UpdateLayeredWindow, WM_APP, WM_CLOSE, WM_DESTROY, WM_PAINT, WNDCLASSW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

const WINDOW_CLASS: &str = "capri-bar-island-window";
const WINDOW_TITLE: &str = "capri-bar";
/// Wakes the event loop when Slint asks for a redraw, so an idle island can sleep
/// with no timer running and still repaint the instant anything changes.
const REDRAW_MESSAGE: u32 = WM_APP + 1;

/// Side of the drawing surface created at startup, in physical pixels.
///
/// It is only what the window is born with: the island's real size comes from
/// `ui/island.slint`, through `WindowAdapter::update_window_properties`, which Slint
/// calls before the window is shown. Keeping the real size out of this file is the
/// point -- there is one place where the island is defined, and it is not Rust.
const PROVISIONAL_SIDE: usize = 16;

/// The island window, its drawing surface and its position on screen.
pub struct IslandWindow {
    hwnd: HWND,
    surface: RefCell<Surface>,
}

/// The pixels Slint draws into: a top-down 32-bit DIB, premultiplied BGRA.
struct Surface {
    memory_dc: HDC,
    bitmap: HBITMAP,
    /// Whatever was selected in the DC before our bitmap. Windows refuses to delete a
    /// bitmap that is still selected, so this is restored first.
    replaced_object: HGDIOBJ,
    pixels: *mut BgraPixel,
    width: usize,
    height: usize,
}

impl Surface {
    fn new(width: usize, height: usize) -> Result<Self, PlatformError> {
        let mut info: BITMAPINFO = unsafe { std::mem::zeroed() };
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32), // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..unsafe { std::mem::zeroed() }
        };

        let memory_dc = unsafe { CreateCompatibleDC(std::ptr::null_mut()) };
        if memory_dc.is_null() {
            return Err(surface_error("CreateCompatibleDC"));
        }

        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let bitmap = unsafe {
            CreateDIBSection(
                memory_dc,
                &info,
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            )
        };
        if bitmap.is_null() || bits.is_null() {
            unsafe { DeleteDC(memory_dc) };
            return Err(surface_error("CreateDIBSection"));
        }

        let replaced_object = unsafe { SelectObject(memory_dc, bitmap as HGDIOBJ) };
        Ok(Self {
            memory_dc,
            bitmap,
            replaced_object,
            pixels: bits as *mut BgraPixel,
            width,
            height,
        })
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            // Deselect first: a selected bitmap cannot be deleted.
            SelectObject(self.memory_dc, self.replaced_object);
            DeleteObject(self.bitmap as HGDIOBJ);
            DeleteDC(self.memory_dc);
        }
    }
}

fn surface_error(what: &str) -> PlatformError {
    PlatformError::Other(format!("{what} failed for the island drawing surface"))
}

impl IslandWindow {
    /// Creates the island window hidden, with the styles described in the module
    /// documentation and a provisional drawing surface.
    pub fn create() -> Result<Rc<Self>, PlatformError> {
        // Without this the process would be DPI-virtualized and the island would be
        // sized and positioned in virtual pixels. It can legitimately fail if the
        // process is already aware, so it is not fatal.
        if unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) } == 0
        {
            eprintln!("capri-bar: could not set per-monitor DPI awareness; sizes may be virtualized");
        }

        let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
        let class_name = wide(WINDOW_CLASS);
        let window_title = wide(WINDOW_TITLE);

        let hwnd = unsafe {
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
            // absent, and it is never added later.
            let style = WS_POPUP;
            let ex_style = WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE;

            CreateWindowExW(
                ex_style,
                class_name.as_ptr(),
                window_title.as_ptr(),
                style,
                0,
                0,
                PROVISIONAL_SIDE as i32,
                PROVISIONAL_SIDE as i32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            )
        };
        if hwnd.is_null() {
            return Err(PlatformError::Other(
                "CreateWindowExW failed for the island window".into(),
            ));
        }

        // From here on the window exists, so every failure path has to destroy it.
        let surface = match Surface::new(PROVISIONAL_SIDE, PROVISIONAL_SIDE) {
            Ok(surface) => surface,
            Err(error) => {
                unsafe { DestroyWindow(hwnd) };
                return Err(error);
            }
        };

        let window = Rc::new(Self {
            hwnd,
            surface: RefCell::new(surface),
        });
        window.centre_at_top_of_primary_monitor();
        Ok(window)
    }

    /// The size of the drawing surface in physical pixels, which is also the size of
    /// the window: the island does not grow by being resized, it grows by being drawn,
    /// and the surface follows what `ui/island.slint` asks for.
    pub fn physical_size(&self) -> (u32, u32) {
        let surface = self.surface.borrow();
        (surface.width as u32, surface.height as u32)
    }

    /// The monitor scale factor of the monitor the window is on.
    pub fn scale_factor(&self) -> f32 {
        (unsafe { GetDpiForWindow(self.hwnd) }) as f32 / 96.0
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

    /// Gives the island a drawing surface of the requested size, keeping it centred.
    ///
    /// Nothing happens when the size is already right, so this is cheap to call from
    /// the places that can change it: a new size coming from `ui/island.slint`, or a
    /// monitor scale factor that changed under us.
    pub fn resize_surface(&self, width: usize, height: usize) -> Result<(), PlatformError> {
        let width = width.max(1);
        let height = height.max(1);
        if self.physical_size() == (width as u32, height as u32) {
            return Ok(());
        }

        let new_surface = Surface::new(width, height)?;
        // Assigning drops the old surface, whose Drop puts its GDI objects back.
        *self.surface.borrow_mut() = new_surface;
        self.centre_at_top_of_primary_monitor();
        Ok(())
    }

    /// Runs `draw` with a line-by-line view over the window pixels, which is how
    /// Slint renders into a surface of ours without any intermediate copy.
    pub fn with_line_buffer<R>(&self, draw: impl FnOnce(IslandLineBuffer<'_>) -> R) -> R {
        let mut surface = self.surface.borrow_mut();
        let surface = &mut *surface;
        // SAFETY: the DIB is owned by this surface and outlives the borrow; the slice
        // never escapes the closure, and this whole module is used from a single thread.
        let pixels =
            unsafe { std::slice::from_raw_parts_mut(surface.pixels, surface.width * surface.height) };
        draw(IslandLineBuffer {
            pixels,
            width: surface.width,
        })
    }

    /// Pushes the rendered pixels to the screen, with their alpha.
    pub fn present(&self) -> Result<(), PlatformError> {
        let surface = self.surface.borrow();
        let (left, top) = self.window_origin();
        let destination = POINT { x: left, y: top };
        let size = SIZE {
            cx: surface.width as i32,
            cy: surface.height as i32,
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
                &destination,
                &size,
                surface.memory_dc,
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

    fn window_origin(&self) -> (i32, i32) {
        let mut rect: windows_sys::Win32::Foundation::RECT = unsafe { std::mem::zeroed() };
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(self.hwnd, &mut rect) };
        (rect.left, rect.top)
    }

    /// Centred horizontally at the top of the primary monitor's work area, so the
    /// island lands below whatever space the desktop already reserves for a bar.
    fn centre_at_top_of_primary_monitor(&self) {
        let (width, height) = self.physical_size();
        unsafe {
            let monitor = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
            let mut info: MONITORINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(monitor, &mut info) == 0 {
                return;
            }
            let work = info.rcWork;
            let x = work.left + ((work.right - work.left) - width as i32) / 2;
            let y = work.top;
            SetWindowPos(
                self.hwnd,
                std::ptr::null_mut(),
                x,
                y,
                width as i32,
                height as i32,
                SWP_NOACTIVATE | SWP_NOZORDER,
            );
        }
    }
}

impl Drop for IslandWindow {
    fn drop(&mut self) {
        unsafe { DestroyWindow(self.hwnd) };
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
