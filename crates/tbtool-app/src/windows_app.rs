use std::{
    collections::HashMap,
    ffi::c_void,
    fs,
    io::Cursor,
    mem::{size_of, zeroed},
    path::{Path, PathBuf},
    ptr::{null, null_mut},
};

use tbtool_core::{
    HardwareCategory, HardwareSnapshot, IniDocument, SkinPackage, ToolCatalog, ToolLauncher,
    ToolTarget, collect_hardware_snapshot,
};
use windows_sys::Win32::{
    Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{
        BeginPaint, BitBlt, CLIP_DEFAULT_PRECIS, CreateCompatibleBitmap, CreateCompatibleDC,
        CreateFontW, CreateSolidBrush, DEFAULT_CHARSET, DEFAULT_PITCH, DeleteDC, DeleteObject,
        DrawTextW, EndPaint, FF_DONTCARE, FW_NORMAL, FillRect, GetStockObject, HBRUSH, HGDIOBJ,
        InvalidateRect, OUT_DEFAULT_PRECIS, PAINTSTRUCT, SRCCOPY, SelectObject, SetBkMode,
        SetTextColor, TRANSPARENT, UpdateWindow, WHITE_BRUSH,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Controls::Dialogs::{
            GetOpenFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST,
            OPENFILENAMEW,
        },
        HiDpi::{
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow,
            SetProcessDpiAwarenessContext,
        },
        WindowsAndMessaging::{
            CREATESTRUCTW, CS_DBLCLKS, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW,
            DefWindowProcW, DestroyWindow, DispatchMessageW, GWLP_USERDATA, GetClientRect,
            GetMessageW, GetWindowLongPtrW, GetWindowRect, HMENU, HTCAPTION, HTCLIENT, IDC_ARROW,
            IDC_HAND, LoadCursorW, MB_DEFBUTTON2, MB_ICONERROR, MB_ICONWARNING, MB_OK, MB_YESNO,
            MSG, MessageBoxW, MoveWindow, PostMessageW, PostQuitMessage, RegisterClassW,
            SW_MINIMIZE, SW_SHOW, SendMessageW, SetCursor, SetWindowLongPtrW, ShowWindow,
            TranslateMessage, WM_APP, WM_CREATE, WM_DESTROY, WM_DPICHANGED, WM_ERASEBKGND,
            WM_GETMINMAXINFO, WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_MOUSEMOVE, WM_NCCREATE,
            WM_NCDESTROY, WM_NCHITTEST, WM_PAINT, WM_SETFONT, WM_SIZE, WNDCLASSW, WS_CHILD,
            WS_CLIPCHILDREN, WS_EX_APPWINDOW, WS_EX_CLIENTEDGE, WS_EX_CONTROLPARENT, WS_HSCROLL,
            WS_OVERLAPPEDWINDOW, WS_POPUP, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
        },
    },
};

const CLASS_NAME: &str = "TbToolRustMainWindow";
const HARDWARE_CLASS_NAME: &str = "TbToolRustHardwareWindow";
const WINDOW_TITLE: &str = "图吧工具箱";
const PASSWORD: &[u8] = b"tulading123";
const BASE_WIDTH: i32 = 1024;
const BASE_HEIGHT: i32 = 600;
const SIDEBAR_WIDTH: i32 = 200;
const SIDEBAR_TOP: i32 = 56;
const SIDEBAR_ROW_HEIGHT: i32 = 40;
const GRID_LEFT: i32 = 216;
const GRID_TOP: i32 = 56;
const GRID_CELL_WIDTH: i32 = 75;
const GRID_CELL_HEIGHT: i32 = 80;
const GRID_COLUMNS: usize = 10;
const MENU_LEFT: i32 = 891;
const SETTINGS_LEFT: i32 = 216;
const SETTINGS_TOP: i32 = 60;
const SETTINGS_ROW_HEIGHT: i32 = 30;
const WM_LOAD_HARDWARE: u32 = WM_APP + 1;
const IDYES: i32 = 6;

const ES_AUTOHSCROLL: u32 = 0x0080;
const ES_MULTILINE: u32 = 0x0004;
const ES_AUTOVSCROLL: u32 = 0x0040;
const ES_READONLY: u32 = 0x0800;

const DT_CENTER: u32 = 0x0001;
const DT_VCENTER: u32 = 0x0004;
const DT_WORDBREAK: u32 = 0x0010;
const DT_SINGLELINE: u32 = 0x0020;
const DT_NOPREFIX: u32 = 0x0800;
const DT_END_ELLIPSIS: u32 = 0x8000;

type GpImage = c_void;
type GpGraphics = c_void;
type GpBrush = c_void;

#[repr(C)]
struct GdiplusStartupInput {
    version: u32,
    debug_event_callback: *const c_void,
    suppress_background_thread: i32,
    suppress_external_codecs: i32,
}

#[repr(C)]
struct Unknown {
    vtable: *const UnknownVtable,
}

#[repr(C)]
struct UnknownVtable {
    query_interface: *const c_void,
    add_ref: *const c_void,
    release: unsafe extern "system" fn(*mut Unknown) -> u32,
}

#[link(name = "gdiplus")]
unsafe extern "system" {
    fn GdiplusStartup(
        token: *mut usize,
        input: *const GdiplusStartupInput,
        output: *mut c_void,
    ) -> i32;
    fn GdiplusShutdown(token: usize);
    fn GdipLoadImageFromStream(stream: *mut Unknown, image: *mut *mut GpImage) -> i32;
    fn GdipDisposeImage(image: *mut GpImage) -> i32;
    fn GdipGetImageWidth(image: *mut GpImage, width: *mut u32) -> i32;
    fn GdipGetImageHeight(image: *mut GpImage, height: *mut u32) -> i32;
    fn GdipCreateFromHDC(dc: *mut c_void, graphics: *mut *mut GpGraphics) -> i32;
    fn GdipDeleteGraphics(graphics: *mut GpGraphics) -> i32;
    fn GdipDrawImageRectI(
        graphics: *mut GpGraphics,
        image: *mut GpImage,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> i32;
    fn GdipCreateSolidFill(color: u32, brush: *mut *mut GpBrush) -> i32;
    fn GdipFillRectangleI(
        graphics: *mut GpGraphics,
        brush: *mut GpBrush,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> i32;
    fn GdipDeleteBrush(brush: *mut GpBrush) -> i32;
}

#[link(name = "shlwapi")]
unsafe extern "system" {
    fn SHCreateMemStream(data: *const u8, length: u32) -> *mut Unknown;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetTickCount64() -> u64;
}

struct GdiplusSession(usize);

impl GdiplusSession {
    fn start() -> Result<Self, std::io::Error> {
        let input = GdiplusStartupInput {
            version: 1,
            debug_event_callback: null(),
            suppress_background_thread: 0,
            suppress_external_codecs: 0,
        };
        let mut token = 0;
        let status = unsafe { GdiplusStartup(&mut token, &input, null_mut()) };
        if status != 0 {
            return Err(std::io::Error::other(format!(
                "GDI+ 初始化失败（状态 {status}）"
            )));
        }
        Ok(Self(token))
    }
}

impl Drop for GdiplusSession {
    fn drop(&mut self) {
        unsafe { GdiplusShutdown(self.0) };
    }
}

struct GdiImage {
    image: *mut GpImage,
    stream: *mut Unknown,
}

impl GdiImage {
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let length = u32::try_from(bytes.len()).ok()?;
        let stream = unsafe { SHCreateMemStream(bytes.as_ptr(), length) };
        if stream.is_null() {
            return None;
        }
        let mut image = null_mut();
        let status = unsafe { GdipLoadImageFromStream(stream, &mut image) };
        if status != 0 || image.is_null() {
            unsafe { ((*(*stream).vtable).release)(stream) };
            return None;
        }
        Some(Self { image, stream })
    }

    fn load(path: &Path) -> Result<Self, std::io::Error> {
        let bytes = fs::read(path)?;
        Self::from_bytes(&bytes)
            .ok_or_else(|| std::io::Error::other(format!("无法解码皮肤图片：{}", path.display())))
    }
}

impl Drop for GdiImage {
    fn drop(&mut self) {
        unsafe {
            GdipDisposeImage(self.image);
            ((*(*self.stream).vtable).release)(self.stream);
        }
    }
}

#[derive(Clone)]
enum PageKind {
    Hardware,
    Tools(Vec<usize>),
    Settings,
}

static SETTINGS_PAGE: PageKind = PageKind::Settings;

struct SidebarItem {
    label: &'static str,
    page: PageKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HitTarget {
    Sidebar(usize),
    Tool(usize),
    Menu,
    Setting(SettingAction),
    Minimize,
    Close,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingAction {
    SilentHardware,
    ChooseSkin,
    InternalImageViewer,
    ToolTips,
    WindowEffects,
}

#[derive(Clone, Copy)]
struct AppSettings {
    silent_hardware: bool,
    internal_image_viewer: bool,
    tool_tips: bool,
    window_effects: bool,
}

impl AppSettings {
    fn from_ini(config: &IniDocument) -> Self {
        Self {
            silent_hardware: ini_bool(config, "静默检测硬件信息", false),
            internal_image_viewer: ini_bool(config, "使用内置图片查看器打开天梯图", true),
            tool_tips: ini_bool(config, "打开工具时显示提示", true),
            window_effects: ini_bool(config, "适配窗口动画和阴影特效", true),
        }
    }
}

enum HardwareState {
    Loading,
    Ready(HardwareSnapshot),
}

struct AppState {
    root: PathBuf,
    config_path: PathBuf,
    config: IniDocument,
    settings: AppSettings,
    catalog: ToolCatalog,
    launcher: ToolLauncher,
    hwnd: HWND,
    background: GdiImage,
    hardware_background: GdiImage,
    hardware_icons: Vec<GdiImage>,
    icon_cache: HashMap<u64, GdiImage>,
    sidebar: Vec<SidebarItem>,
    active_sidebar: usize,
    settings_visible: bool,
    visible_tools: Vec<(usize, usize)>,
    selected_tool: Option<usize>,
    hover: Option<HitTarget>,
    status: String,
    hardware: HardwareState,
}

struct HardwareWindowState {
    report: String,
    edit: HWND,
    font: HGDIOBJ,
}

impl AppState {
    fn new(
        root: &Path,
        catalog: ToolCatalog,
        launcher: ToolLauncher,
    ) -> Result<Self, std::io::Error> {
        let config_path = root.join("Config.ini");
        let config = IniDocument::parse(&fs::read(&config_path)?).map_err(io_error)?;
        let settings = AppSettings::from_ini(&config);
        let user_skin = root.join("skin/user");
        let background = GdiImage::load(&user_skin.join("默认底图.png"))?;
        let hardware_background = GdiImage::load(&user_skin.join("硬件信息底图.png"))?;
        let hardware_icons = ["型号信息.png", "系统信息.png", "运行时间.png"]
            .into_iter()
            .map(|name| GdiImage::load(&user_skin.join(name)))
            .collect::<Result<Vec<_>, _>>()?;
        let sidebar = build_sidebar(&catalog);
        Ok(Self {
            root: root.to_path_buf(),
            config_path,
            config,
            settings,
            catalog,
            launcher,
            hwnd: null_mut(),
            background,
            hardware_background,
            hardware_icons,
            icon_cache: HashMap::new(),
            sidebar,
            active_sidebar: 0,
            settings_visible: false,
            visible_tools: Vec::new(),
            selected_tool: None,
            hover: None,
            status: "硬件信息正在读取中…".to_owned(),
            hardware: HardwareState::Loading,
        })
    }

    fn current_page(&self) -> &PageKind {
        if self.settings_visible {
            &SETTINGS_PAGE
        } else {
            &self.sidebar[self.active_sidebar].page
        }
    }

    unsafe fn select_sidebar(&mut self, index: usize) {
        if index >= self.sidebar.len() {
            return;
        }
        self.settings_visible = false;
        self.active_sidebar = index;
        self.selected_tool = None;
        self.visible_tools.clear();
        match self.sidebar[index].page.clone() {
            PageKind::Hardware => {
                self.status = match self.hardware {
                    HardwareState::Loading => "硬件信息正在读取中…".to_owned(),
                    HardwareState::Ready(_) => "硬件信息".to_owned(),
                };
            }
            PageKind::Tools(categories) => {
                for category_index in categories {
                    if let Some(category) = self.catalog.categories.get(category_index) {
                        self.visible_tools.extend(
                            (0..category.tools.len())
                                .map(|tool_index| (category_index, tool_index)),
                        );
                    }
                }
                self.status = "提示：单击选择工具可查看工具说明，双击可启动工具。".to_owned();
            }
            PageKind::Settings => unreachable!("settings is opened from the title-bar menu"),
        }
        unsafe { InvalidateRect(self.hwnd, null(), 0) };
    }

    unsafe fn show_settings(&mut self) {
        self.settings_visible = true;
        self.selected_tool = None;
        self.visible_tools.clear();
        self.status = "选项设置".to_owned();
        unsafe { InvalidateRect(self.hwnd, null(), 0) };
    }

    fn tool_at(&self, visible_index: usize) -> Option<&tbtool_core::ToolEntry> {
        let (category, tool) = *self.visible_tools.get(visible_index)?;
        self.catalog.categories.get(category)?.tools.get(tool)
    }

    fn select_tool(&mut self, visible_index: usize) {
        let Some(tool) = self.tool_at(visible_index) else {
            return;
        };
        self.status = if tool.description.trim().is_empty() {
            format!("{}：双击启动。", tool.name)
        } else {
            format!("{}：{}", tool.name, tool.description.trim())
        };
        self.selected_tool = Some(visible_index);
    }

    unsafe fn activate_tool(&mut self, visible_index: usize) {
        let Some(tool) = self.tool_at(visible_index) else {
            return;
        };
        let name = tool.name.clone();
        let target = tool.target.clone();
        match target {
            ToolTarget::BuiltIn { name: action } => unsafe { self.run_builtin(&action) },
            target => match self.launcher.launch(&target) {
                Ok(launched) => {
                    self.status = launched.process_id.map_or_else(
                        || format!("已通过系统关联打开 {name}"),
                        |process_id| format!("已启动 {name} · PID {process_id}"),
                    );
                }
                Err(error) => unsafe {
                    message_box(
                        self.hwnd,
                        &format!("无法启动 {name}\r\n\r\n{error}"),
                        "启动失败",
                        MB_OK | MB_ICONERROR,
                    );
                },
            },
        }
        unsafe { InvalidateRect(self.hwnd, null(), 0) };
    }

    fn icon_for(&mut self, visible_index: usize) -> Option<*mut GpImage> {
        let (category_index, tool_index) = *self.visible_tools.get(visible_index)?;
        let key = ((category_index as u64) << 32) | tool_index as u64;
        if !self.icon_cache.contains_key(&key) {
            let bytes = self
                .catalog
                .categories
                .get(category_index)?
                .tools
                .get(tool_index)
                .map(|tool| {
                    if !tool.icon.is_empty() {
                        tool.icon.clone()
                    } else if !tool.icon_40.is_empty() {
                        tool.icon_40.clone()
                    } else {
                        tool.icon_48.clone()
                    }
                })?;
            if let Some(image) = GdiImage::from_bytes(&bytes) {
                self.icon_cache.insert(key, image);
            }
        }
        self.icon_cache.get(&key).map(|image| image.image)
    }

    fn logical_point(&self, x: i32, y: i32) -> (i32, i32) {
        let mut rect: RECT = unsafe { zeroed() };
        unsafe { GetClientRect(self.hwnd, &mut rect) };
        let width = rect.right.max(1);
        let height = rect.bottom.max(1);
        (x * BASE_WIDTH / width, y * BASE_HEIGHT / height)
    }

    fn hit_test(&self, x: i32, y: i32) -> Option<HitTarget> {
        let (x, y) = self.logical_point(x, y);
        if (971..=1010).contains(&x) && (8..=48).contains(&y) {
            return Some(HitTarget::Close);
        }
        if (931..=970).contains(&x) && (8..=48).contains(&y) {
            return Some(HitTarget::Minimize);
        }
        if (MENU_LEFT..=930).contains(&x) && (8..=48).contains(&y) {
            return Some(HitTarget::Menu);
        }
        if x < SIDEBAR_WIDTH && y >= SIDEBAR_TOP {
            let index = ((y - SIDEBAR_TOP) / SIDEBAR_ROW_HEIGHT) as usize;
            if index < self.sidebar.len() {
                return Some(HitTarget::Sidebar(index));
            }
        }
        if self.settings_visible && x >= SETTINGS_LEFT && y >= SETTINGS_TOP {
            let index = ((y - SETTINGS_TOP) / SETTINGS_ROW_HEIGHT) as usize;
            let action = match index {
                0 => SettingAction::SilentHardware,
                1 => SettingAction::ChooseSkin,
                2 => SettingAction::InternalImageViewer,
                3 => SettingAction::ToolTips,
                4 => SettingAction::WindowEffects,
                _ => return None,
            };
            return Some(HitTarget::Setting(action));
        }
        if matches!(self.current_page(), PageKind::Tools(_)) && x >= GRID_LEFT && y >= GRID_TOP {
            let column = ((x - GRID_LEFT) / GRID_CELL_WIDTH) as usize;
            let row = ((y - GRID_TOP) / GRID_CELL_HEIGHT) as usize;
            if column < GRID_COLUMNS {
                let index = row * GRID_COLUMNS + column;
                if index < self.visible_tools.len() {
                    return Some(HitTarget::Tool(index));
                }
            }
        }
        None
    }

    unsafe fn mouse_up(&mut self, x: i32, y: i32) {
        match self.hit_test(x, y) {
            Some(HitTarget::Close) => unsafe {
                DestroyWindow(self.hwnd);
            },
            Some(HitTarget::Minimize) => unsafe {
                ShowWindow(self.hwnd, SW_MINIMIZE);
            },
            Some(HitTarget::Menu) => unsafe { self.show_settings() },
            Some(HitTarget::Sidebar(index)) => unsafe { self.select_sidebar(index) },
            Some(HitTarget::Setting(action)) => unsafe { self.activate_setting(action) },
            Some(HitTarget::Tool(index)) => {
                self.select_tool(index);
                unsafe { InvalidateRect(self.hwnd, null(), 0) };
            }
            None => {}
        }
    }

    unsafe fn activate_setting(&mut self, action: SettingAction) {
        if action == SettingAction::ChooseSkin {
            unsafe { self.choose_skin() };
            return;
        }

        let (key, current) = match action {
            SettingAction::SilentHardware => ("静默检测硬件信息", self.settings.silent_hardware),
            SettingAction::InternalImageViewer => (
                "使用内置图片查看器打开天梯图",
                self.settings.internal_image_viewer,
            ),
            SettingAction::ToolTips => ("打开工具时显示提示", self.settings.tool_tips),
            SettingAction::WindowEffects => {
                ("适配窗口动画和阴影特效", self.settings.window_effects)
            }
            SettingAction::ChooseSkin => unreachable!(),
        };
        let next = !current;
        if action == SettingAction::WindowEffects
            && next
            && unsafe {
                message_box(
                    self.hwnd,
                    "该功能处于测试阶段，不保证稳定。如果开启后程序出现报错、闪退或无法启动，可删除 Config.ini 恢复默认设置。\r\n\r\n是否确认开启？",
                    "适配窗口动画和阴影特效 (Beta)",
                    MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2,
                )
            } != IDYES
        {
            return;
        }

        let mut config = self.config.clone();
        config.set("设置", key, bool_text(next));
        let result = config
            .to_bytes()
            .map_err(io_error)
            .and_then(|bytes| fs::write(&self.config_path, bytes));
        if let Err(error) = result {
            unsafe {
                message_box(
                    self.hwnd,
                    &format!("无法保存选项设置\r\n\r\n{error}"),
                    "保存失败",
                    MB_OK | MB_ICONERROR,
                )
            };
            return;
        }

        self.config = config;
        match action {
            SettingAction::SilentHardware => self.settings.silent_hardware = next,
            SettingAction::InternalImageViewer => self.settings.internal_image_viewer = next,
            SettingAction::ToolTips => self.settings.tool_tips = next,
            SettingAction::WindowEffects => self.settings.window_effects = next,
            SettingAction::ChooseSkin => unreachable!(),
        }
        unsafe { InvalidateRect(self.hwnd, null(), 0) };
    }

    unsafe fn choose_skin(&mut self) {
        let Some(path) = (unsafe { open_skin_file(self.hwnd, &self.root.join("skin")) }) else {
            return;
        };
        if let Err(error) = self.apply_skin(&path) {
            unsafe {
                message_box(
                    self.hwnd,
                    &format!("无法应用所选皮肤\r\n\r\n{error}"),
                    "皮肤加载失败",
                    MB_OK | MB_ICONERROR,
                )
            };
            return;
        }
        unsafe { InvalidateRect(self.hwnd, null(), 0) };
    }

    fn apply_skin(&mut self, path: &Path) -> Result<(), std::io::Error> {
        let package = SkinPackage::from_zip(Cursor::new(fs::read(path)?)).map_err(io_error)?;
        let asset = |name: &'static str| {
            package
                .asset(name)
                .ok_or_else(|| std::io::Error::other(format!("皮肤缺少资源：{name}")))
        };

        let background = GdiImage::from_bytes(asset("默认底图.png")?)
            .ok_or_else(|| std::io::Error::other("无法解码皮肤默认底图"))?;
        let hardware_background = GdiImage::from_bytes(asset("硬件信息底图.png")?)
            .ok_or_else(|| std::io::Error::other("无法解码皮肤硬件信息底图"))?;
        let hardware_icons = ["型号信息.png", "系统信息.png", "运行时间.png"]
            .into_iter()
            .map(|name| {
                GdiImage::from_bytes(asset(name)?)
                    .ok_or_else(|| std::io::Error::other(format!("无法解码皮肤资源：{name}")))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let user_skin = self.root.join("skin/user");
        for name in [
            "默认底图.png",
            "运行时间.png",
            "系统信息.png",
            "硬件信息底图.png",
            "控制按钮.png",
            "型号信息.png",
            "列表按钮.png",
        ] {
            fs::write(user_skin.join(name), asset(name)?)?;
        }
        fs::write(
            user_skin.join("Config.ini"),
            package.config.to_bytes().map_err(io_error)?,
        )?;

        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "自定义.skin".to_owned());
        let skin_name = package
            .manifest
            .name
            .clone()
            .or_else(|| {
                path.file_stem()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "自定义".to_owned());
        let author = package.manifest.author.clone().unwrap_or_default();
        let mut config = self.config.clone();
        config.set("皮肤", "文件名", file_name);
        config.set("皮肤", "皮肤名", skin_name);
        config.set("皮肤", "作者", author);
        fs::write(&self.config_path, config.to_bytes().map_err(io_error)?)?;

        self.background = background;
        self.hardware_background = hardware_background;
        self.hardware_icons = hardware_icons;
        self.config = config;
        Ok(())
    }

    unsafe fn mouse_double_click(&mut self, x: i32, y: i32) {
        match self.hit_test(x, y) {
            Some(HitTarget::Tool(index)) => unsafe { self.activate_tool(index) },
            _ if matches!(self.current_page(), PageKind::Hardware) => {
                if let HardwareState::Ready(snapshot) = &self.hardware {
                    let _ = unsafe {
                        show_report_window(self.hwnd, "硬件检测报告", snapshot.to_text())
                    };
                }
            }
            _ => {}
        }
    }

    unsafe fn load_hardware(&mut self) {
        self.hardware = match collect_hardware_snapshot() {
            Ok(snapshot) => HardwareState::Ready(snapshot),
            Err(error) => HardwareState::Ready(HardwareSnapshot {
                computer_name: "读取失败".to_owned(),
                sections: Vec::new(),
                warnings: vec![error.to_string()],
            }),
        };
        if matches!(self.current_page(), PageKind::Hardware) {
            self.status = "硬件信息（双击可查看完整报告）".to_owned();
        }
        unsafe { InvalidateRect(self.hwnd, null(), 0) };
    }

    unsafe fn paint(&mut self, target_dc: *mut c_void) {
        let mut client: RECT = unsafe { zeroed() };
        unsafe { GetClientRect(self.hwnd, &mut client) };
        let width = client.right.max(1);
        let height = client.bottom.max(1);
        let sx = |value: i32| value * width / BASE_WIDTH;
        let sy = |value: i32| value * height / BASE_HEIGHT;

        let buffer_dc = unsafe { CreateCompatibleDC(target_dc) };
        let bitmap = unsafe { CreateCompatibleBitmap(target_dc, width, height) };
        let old_bitmap = unsafe { SelectObject(buffer_dc, bitmap as HGDIOBJ) };
        let fallback = unsafe { CreateSolidBrush(rgb(27, 129, 193)) };
        unsafe { FillRect(buffer_dc, &client, fallback) };

        let mut graphics = null_mut();
        if unsafe { GdipCreateFromHDC(buffer_dc, &mut graphics) } == 0 {
            unsafe {
                GdipDrawImageRectI(graphics, self.background.image, 0, 0, width, height);
            }
            if matches!(self.current_page(), PageKind::Hardware) {
                unsafe {
                    GdipDrawImageRectI(
                        graphics,
                        self.hardware_background.image,
                        0,
                        0,
                        width,
                        height,
                    );
                }
            }
            let mut selection_brush = null_mut();
            if !self.settings_visible
                && unsafe { GdipCreateSolidFill(0x20ff_ffff, &mut selection_brush) } == 0
            {
                unsafe {
                    GdipFillRectangleI(
                        graphics,
                        selection_brush,
                        0,
                        sy(SIDEBAR_TOP + self.active_sidebar as i32 * SIDEBAR_ROW_HEIGHT),
                        sx(SIDEBAR_WIDTH),
                        sy(SIDEBAR_ROW_HEIGHT),
                    );
                    GdipDeleteBrush(selection_brush);
                }
            }

            if matches!(self.current_page(), PageKind::Hardware) {
                for (index, icon) in self.hardware_icons.iter().enumerate() {
                    unsafe {
                        GdipDrawImageRectI(
                            graphics,
                            icon.image,
                            sx(244 + index as i32 * 272),
                            sy(82),
                            sx(16),
                            sy(16),
                        );
                    }
                }
            }

            if matches!(self.current_page(), PageKind::Tools(_)) {
                let tool_count = self.visible_tools.len();
                for index in 0..tool_count {
                    let column = (index % GRID_COLUMNS) as i32;
                    let row = (index / GRID_COLUMNS) as i32;
                    let cell_x = GRID_LEFT + column * GRID_CELL_WIDTH;
                    let cell_y = GRID_TOP + row * GRID_CELL_HEIGHT;
                    if self.selected_tool == Some(index) {
                        let brush = unsafe { CreateSolidBrush(rgb(37, 159, 205)) };
                        let rect = scaled_rect(
                            cell_x + 3,
                            cell_y + 3,
                            GRID_CELL_WIDTH - 6,
                            GRID_CELL_HEIGHT - 6,
                            width,
                            height,
                        );
                        unsafe {
                            FillRect(buffer_dc, &rect, brush);
                            DeleteObject(brush as HGDIOBJ);
                        }
                    }
                    if let Some(icon) = self.icon_for(index) {
                        let mut icon_width = 32;
                        let mut icon_height = 32;
                        unsafe {
                            GdipGetImageWidth(icon, &mut icon_width);
                            GdipGetImageHeight(icon, &mut icon_height);
                        }
                        let icon_width = icon_width.min(40) as i32;
                        let icon_height = icon_height.min(40) as i32;
                        unsafe {
                            GdipDrawImageRectI(
                                graphics,
                                icon,
                                sx(cell_x + (GRID_CELL_WIDTH - icon_width) / 2),
                                sy(cell_y + 8 + (40 - icon_height) / 2),
                                sx(icon_width),
                                sy(icon_height),
                            );
                        }
                    }
                }
            }
            unsafe { GdipDeleteGraphics(graphics) };
        }

        unsafe { SetBkMode(buffer_dc, TRANSPARENT as i32) };
        let scale = (width as f32 / BASE_WIDTH as f32)
            .min(height as f32 / BASE_HEIGHT as f32)
            .max(0.5);
        let title_font = unsafe { create_ui_font_px((16.0 * scale) as i32, 500) };
        let nav_font = unsafe { create_ui_font_px((14.0 * scale) as i32, FW_NORMAL as i32) };
        let small_font = unsafe { create_ui_font_px((12.0 * scale) as i32, FW_NORMAL as i32) };
        let section_font = unsafe { create_ui_font_px((17.0 * scale) as i32, 500) };

        unsafe { SetTextColor(buffer_dc, rgb(255, 255, 255)) };
        let old_font = unsafe { SelectObject(buffer_dc, title_font) };
        unsafe {
            draw_text(
                buffer_dc,
                WINDOW_TITLE,
                scaled_rect(0, 8, 200, 40, width, height),
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
        }
        unsafe { SelectObject(buffer_dc, nav_font) };
        for (index, item) in self.sidebar.iter().enumerate() {
            unsafe {
                draw_text(
                    buffer_dc,
                    item.label,
                    scaled_rect(
                        0,
                        SIDEBAR_TOP + index as i32 * SIDEBAR_ROW_HEIGHT,
                        SIDEBAR_WIDTH,
                        SIDEBAR_ROW_HEIGHT,
                        width,
                        height,
                    ),
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                );
            }
        }
        unsafe { SelectObject(buffer_dc, small_font) };
        unsafe {
            draw_text(
                buffer_dc,
                "Version : 2026.01",
                scaled_rect(0, 570, 200, 30, width, height),
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
        }
        unsafe { SelectObject(buffer_dc, nav_font) };
        unsafe {
            draw_text(
                buffer_dc,
                &self.status,
                scaled_rect(225, 12, 660, 32, width, height),
                DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
            );
            draw_text(
                buffer_dc,
                "≡",
                scaled_rect(899, 8, 32, 40, width, height),
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
        }

        match self.current_page() {
            PageKind::Tools(_) => {
                unsafe { SelectObject(buffer_dc, small_font) };
                let names = (0..self.visible_tools.len())
                    .map(|index| self.tool_at(index).map(|tool| tool.name.clone()))
                    .collect::<Vec<_>>();
                for (index, name) in names.into_iter().enumerate() {
                    let Some(name) = name else { continue };
                    let column = (index % GRID_COLUMNS) as i32;
                    let row = (index / GRID_COLUMNS) as i32;
                    unsafe {
                        draw_text(
                            buffer_dc,
                            &name,
                            scaled_rect(
                                GRID_LEFT + column * GRID_CELL_WIDTH + 2,
                                GRID_TOP + row * GRID_CELL_HEIGHT + 52,
                                GRID_CELL_WIDTH - 4,
                                35,
                                width,
                                height,
                            ),
                            DT_CENTER | DT_WORDBREAK | DT_END_ELLIPSIS | DT_NOPREFIX,
                        );
                    }
                }
            }
            PageKind::Hardware => unsafe {
                self.paint_hardware(buffer_dc, width, height, nav_font, section_font, small_font)
            },
            PageKind::Settings => unsafe {
                self.paint_settings(buffer_dc, width, height, nav_font)
            },
        }

        unsafe {
            BitBlt(target_dc, 0, 0, width, height, buffer_dc, 0, 0, SRCCOPY);
            SelectObject(buffer_dc, old_font);
            SelectObject(buffer_dc, old_bitmap);
            DeleteObject(bitmap as HGDIOBJ);
            DeleteObject(fallback as HGDIOBJ);
            DeleteObject(title_font);
            DeleteObject(nav_font);
            DeleteObject(small_font);
            DeleteObject(section_font);
            DeleteDC(buffer_dc);
        }
    }

    unsafe fn paint_hardware(
        &self,
        dc: *mut c_void,
        width: i32,
        height: i32,
        font: HGDIOBJ,
        section_font: HGDIOBJ,
        small_font: HGDIOBJ,
    ) {
        let HardwareState::Ready(snapshot) = &self.hardware else {
            unsafe { SelectObject(dc, section_font) };
            unsafe {
                draw_text(
                    dc,
                    "正在读取硬件信息…",
                    scaled_rect(215, 230, 792, 80, width, height),
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                )
            };
            return;
        };
        let model = hardware_name(snapshot, HardwareCategory::System, "整机信息")
            .unwrap_or_else(|| snapshot.computer_name.clone());
        let system = hardware_name(snapshot, HardwareCategory::System, "操作系统")
            .unwrap_or_else(|| "Windows".to_owned());
        let uptime = uptime_text();
        let cards = [
            ("型号信息", model),
            ("系统信息", system),
            ("运行时间", uptime),
        ];
        for (index, (title, value)) in cards.into_iter().enumerate() {
            let x = 215 + index as i32 * 272;
            unsafe { SetTextColor(dc, rgb(241, 253, 255)) };
            unsafe { SelectObject(dc, section_font) };
            unsafe {
                draw_text(
                    dc,
                    title,
                    scaled_rect(x + 58, 74, 180, 30, width, height),
                    DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                )
            };
            unsafe { SelectObject(dc, font) };
            unsafe {
                draw_text(
                    dc,
                    &value,
                    scaled_rect(x + 22, 116, 218, 40, width, height),
                    DT_CENTER | DT_VCENTER | DT_WORDBREAK | DT_END_ELLIPSIS | DT_NOPREFIX,
                )
            };
        }

        unsafe { SelectObject(dc, section_font) };
        unsafe {
            draw_text(
                dc,
                "详细信息",
                scaled_rect(248, 214, 180, 32, width, height),
                DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            )
        };
        unsafe { SelectObject(dc, font) };
        let details = hardware_details(snapshot);
        unsafe {
            draw_text(
                dc,
                &details,
                scaled_rect(248, 255, 730, 300, width, height),
                DT_WORDBREAK | DT_END_ELLIPSIS | DT_NOPREFIX,
            )
        };
        if !snapshot.warnings.is_empty() {
            unsafe { SelectObject(dc, small_font) };
            unsafe {
                draw_text(
                    dc,
                    &format!(
                        "检测提示：{} 项信息未能读取，双击查看完整报告。",
                        snapshot.warnings.len()
                    ),
                    scaled_rect(248, 548, 730, 22, width, height),
                    DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
                )
            };
        }
    }

    unsafe fn paint_settings(&self, dc: *mut c_void, width: i32, height: i32, font: HGDIOBJ) {
        let options = [
            option_text(self.settings.silent_hardware, "启动时静默检测硬件信息"),
            "【●】选择皮肤".to_owned(),
            option_text(
                self.settings.internal_image_viewer,
                "使用内置图片查看器打开天梯图",
            ),
            option_text(self.settings.tool_tips, "打开工具时显示提示"),
            option_text(
                self.settings.window_effects,
                "适配窗口动画和阴影特效 (Beta)",
            ),
        ];
        unsafe {
            SelectObject(dc, font);
            for (index, option) in options.iter().enumerate() {
                draw_text(
                    dc,
                    option,
                    scaled_rect(
                        SETTINGS_LEFT,
                        SETTINGS_TOP + index as i32 * SETTINGS_ROW_HEIGHT,
                        700,
                        SETTINGS_ROW_HEIGHT,
                        width,
                        height,
                    ),
                    DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                );
            }
        }
    }

    unsafe fn run_builtin(&mut self, action: &str) {
        match action {
            "新手指引" => {
                let report = "图吧工具箱新手指引\r\n\r\n一、硬件信息\r\n在左侧选择“硬件信息”可查看处理器、主板、内存、显卡、磁盘、显示器、网络与音频信息。\r\n\r\n二、选择工具\r\n单击工具可查看说明，双击工具可启动。\r\n\r\n三、稳定性测试\r\n烤机和压力测试会显著增加功耗与温度，请持续观察温度。\r\n";
                let _ = unsafe { show_report_window(self.hwnd, "新手指引", report.to_owned()) };
            }
            "坏点与漏光测试" => {
                if let Err(error) = unsafe { crate::display_test::show(self.hwnd) } {
                    unsafe {
                        message_box(
                            self.hwnd,
                            &error.to_string(),
                            "无法启动屏幕测试",
                            MB_OK | MB_ICONERROR,
                        )
                    };
                }
            }
            "一键烤鸡" => unsafe { self.run_combined_stress_test() },
            _ => unsafe {
                message_box(
                    self.hwnd,
                    &format!("未知内置功能：{action}"),
                    "内置功能错误",
                    MB_OK | MB_ICONERROR,
                );
            },
        }
    }

    unsafe fn run_combined_stress_test(&mut self) {
        let choice = unsafe {
            message_box(
                self.hwnd,
                "即将同时启动 Prime95 CPU 压力测试和 FurMark 显卡压力测试。\r\n\r\n该操作会快速提高整机功耗与温度，请确认散热和电源状态正常。",
                "确认一键烤机",
                MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2,
            )
        };
        if choice != IDYES {
            self.status = "已取消一键烤机".to_owned();
            return;
        }
        let targets = [
            (
                "Prime95",
                ToolTarget::Executable {
                    path: PathBuf::from("tools/处理器工具/Prime95/start.bat"),
                    working_directory: PathBuf::from("tools/处理器工具/Prime95"),
                },
            ),
            (
                "FurMark",
                ToolTarget::Executable {
                    path: PathBuf::from("tools/烤鸡工具/FurMark/start.bat"),
                    working_directory: PathBuf::from("tools/烤鸡工具/FurMark"),
                },
            ),
        ];
        let mut started = Vec::new();
        for (name, target) in targets {
            match self.launcher.launch(&target) {
                Ok(_) => started.push(name),
                Err(error) => {
                    unsafe {
                        message_box(
                            self.hwnd,
                            &format!(
                                "已启动：{}\r\n\r\n{name} 启动失败：{error}",
                                started.join("、")
                            ),
                            "一键烤机未完全启动",
                            MB_OK | MB_ICONERROR,
                        )
                    };
                    return;
                }
            }
        }
        self.status = "已启动 Prime95 与 FurMark".to_owned();
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let root = locate_package_root()?;
    let catalog = ToolCatalog::load(&root, PASSWORD)?;
    let launcher = ToolLauncher::new(&root)?;

    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    let _gdiplus = GdiplusSession::start()?;
    let mut state = Box::new(AppState::new(&root, catalog, launcher)?);

    unsafe {
        let instance = GetModuleHandleW(null());
        let class_name = wide(CLASS_NAME);
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: null_mut(),
            lpszClassName: class_name.as_ptr(),
            ..zeroed()
        };
        if RegisterClassW(&class) == 0 {
            return Err(std::io::Error::last_os_error().into());
        }

        let dpi = 96u32;
        let title = wide(WINDOW_TITLE);
        let state_ptr = state.as_mut() as *mut AppState;
        let hwnd = CreateWindowExW(
            WS_EX_APPWINDOW,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_POPUP | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            BASE_WIDTH * dpi as i32 / 96,
            BASE_HEIGHT * dpi as i32 / 96,
            null_mut(),
            null_mut(),
            instance,
            state_ptr.cast(),
        );
        if hwnd.is_null() {
            return Err(std::io::Error::last_os_error().into());
        }
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);
        PostMessageW(hwnd, WM_LOAD_HARDWARE, 0, 0);

        let mut message: MSG = zeroed();
        while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
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
        let state = unsafe { (*create).lpCreateParams as *mut AppState };
        unsafe {
            (*state).hwnd = hwnd;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize);
        }
    }
    let state = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState };

    match message {
        WM_CREATE => 0,
        WM_LOAD_HARDWARE => {
            if !state.is_null() {
                unsafe { (*state).load_hardware() };
            }
            0
        }
        WM_SIZE => {
            unsafe { InvalidateRect(hwnd, null(), 0) };
            0
        }
        WM_DPICHANGED => {
            let suggested = lparam as *const RECT;
            if !suggested.is_null() {
                let rect = unsafe { *suggested };
                unsafe {
                    MoveWindow(
                        hwnd,
                        rect.left,
                        rect.top,
                        rect.right - rect.left,
                        rect.bottom - rect.top,
                        1,
                    )
                };
            }
            0
        }
        WM_MOUSEMOVE => {
            if !state.is_null() {
                let (x, y) = client_point(lparam);
                let new_hover = unsafe { (*state).hit_test(x, y) };
                if new_hover != unsafe { (*state).hover } {
                    unsafe {
                        (*state).hover = new_hover;
                        InvalidateRect(hwnd, null(), 0);
                    }
                }
                let cursor = if new_hover.is_some() {
                    IDC_HAND
                } else {
                    IDC_ARROW
                };
                unsafe { SetCursor(LoadCursorW(null_mut(), cursor)) };
            }
            0
        }
        WM_LBUTTONUP => {
            if !state.is_null() {
                let (x, y) = client_point(lparam);
                unsafe { (*state).mouse_up(x, y) };
            }
            0
        }
        WM_LBUTTONDBLCLK => {
            if !state.is_null() {
                let (x, y) = client_point(lparam);
                unsafe { (*state).mouse_double_click(x, y) };
            }
            0
        }
        WM_NCHITTEST => {
            if state.is_null() {
                return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
            }
            let screen_x = signed_low_word(lparam);
            let screen_y = signed_high_word(lparam);
            let mut window: RECT = unsafe { zeroed() };
            unsafe { GetWindowRect(hwnd, &mut window) };
            let (x, y) =
                unsafe { (*state).logical_point(screen_x - window.left, screen_y - window.top) };
            if y < 56 && x < MENU_LEFT {
                HTCAPTION as LRESULT
            } else {
                HTCLIENT as LRESULT
            }
        }
        WM_GETMINMAXINFO => {
            if lparam != 0 {
                let limits = lparam as *mut windows_sys::Win32::UI::WindowsAndMessaging::MINMAXINFO;
                let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96) as i32;
                unsafe {
                    (*limits).ptMinTrackSize.x = BASE_WIDTH * dpi / 96;
                    (*limits).ptMinTrackSize.y = BASE_HEIGHT * dpi / 96;
                    (*limits).ptMaxTrackSize = (*limits).ptMinTrackSize;
                }
            }
            0
        }
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            let mut paint: PAINTSTRUCT = unsafe { zeroed() };
            let dc = unsafe { BeginPaint(hwnd, &mut paint) };
            if !state.is_null() {
                unsafe { (*state).paint(dc) };
            }
            unsafe { EndPaint(hwnd, &paint) };
            0
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        WM_NCDESTROY => {
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn build_sidebar(catalog: &ToolCatalog) -> Vec<SidebarItem> {
    let category = |name: &str| {
        catalog
            .categories
            .iter()
            .position(|item| item.name.eq_ignore_ascii_case(name))
            .into_iter()
            .collect::<Vec<_>>()
    };
    vec![
        SidebarItem {
            label: "硬件信息",
            page: PageKind::Hardware,
        },
        SidebarItem {
            label: "CPU工具",
            page: PageKind::Tools(category("CPU工具")),
        },
        SidebarItem {
            label: "主板工具",
            page: PageKind::Tools(category("主板工具")),
        },
        SidebarItem {
            label: "内存工具",
            page: PageKind::Tools(category("内存工具")),
        },
        SidebarItem {
            label: "显卡工具",
            page: PageKind::Tools(category("显卡工具")),
        },
        SidebarItem {
            label: "硬盘工具",
            page: PageKind::Tools(category("硬盘工具")),
        },
        SidebarItem {
            label: "屏幕工具",
            page: PageKind::Tools(category("屏幕工具")),
        },
        SidebarItem {
            label: "综合检测",
            page: PageKind::Tools(category("综合检测")),
        },
        SidebarItem {
            label: "外设工具",
            page: PageKind::Tools(category("外设工具")),
        },
        SidebarItem {
            label: "烤鸡工具",
            page: PageKind::Tools(category("烤鸡工具")),
        },
        SidebarItem {
            label: "游戏工具",
            page: PageKind::Tools(category("游戏工具")),
        },
        SidebarItem {
            label: "其他工具",
            page: PageKind::Tools(category("其他工具")),
        },
    ]
}

fn ini_bool(config: &IniDocument, key: &str, default: bool) -> bool {
    config
        .get("设置", key)
        .map(str::trim)
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "真" | "true" | "1" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

const fn bool_text(value: bool) -> &'static str {
    if value { "真" } else { "假" }
}

fn option_text(enabled: bool, label: &str) -> String {
    format!("【{}】{label}", if enabled { "●" } else { "  " })
}

fn io_error(error: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::other(error.to_string())
}

fn hardware_name(
    snapshot: &HardwareSnapshot,
    category: HardwareCategory,
    preferred_title: &str,
) -> Option<String> {
    snapshot
        .sections
        .iter()
        .find(|section| section.category == category && section.title == preferred_title)
        .or_else(|| {
            snapshot
                .sections
                .iter()
                .find(|section| section.category == category)
        })
        .and_then(|section| section.devices.first())
        .map(|device| device.name.trim().to_owned())
        .filter(|name| !name.is_empty())
}

fn hardware_names(snapshot: &HardwareSnapshot, category: HardwareCategory) -> String {
    let mut names = snapshot
        .sections
        .iter()
        .filter(|section| section.category == category)
        .flat_map(|section| section.devices.iter())
        .map(|device| device.name.trim())
        .filter(|name| !name.is_empty())
        .take(4)
        .collect::<Vec<_>>();
    names.dedup();
    if names.is_empty() {
        "未检测到".to_owned()
    } else {
        names.join(" / ")
    }
}

fn hardware_details(snapshot: &HardwareSnapshot) -> String {
    [
        ("处理器", HardwareCategory::Processor),
        ("主板", HardwareCategory::Mainboard),
        ("内存", HardwareCategory::Memory),
        ("显卡", HardwareCategory::Graphics),
        ("显示器", HardwareCategory::Display),
        ("硬盘", HardwareCategory::Storage),
        ("声卡", HardwareCategory::Audio),
        ("网卡", HardwareCategory::Network),
    ]
    .into_iter()
    .map(|(label, category)| format!("{label}：\t{}", hardware_names(snapshot, category)))
    .collect::<Vec<_>>()
    .join("\r\n")
}

fn uptime_text() -> String {
    let seconds = unsafe { GetTickCount64() } / 1000;
    let days = seconds / 86_400;
    let hours = (seconds / 3_600) % 24;
    let minutes = (seconds / 60) % 60;
    let seconds = seconds % 60;
    format!("{days}天{hours}小时{minutes}分钟{seconds}秒")
}

fn scaled_rect(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    client_width: i32,
    client_height: i32,
) -> RECT {
    RECT {
        left: x * client_width / BASE_WIDTH,
        top: y * client_height / BASE_HEIGHT,
        right: (x + width) * client_width / BASE_WIDTH,
        bottom: (y + height) * client_height / BASE_HEIGHT,
    }
}

unsafe fn draw_text(dc: *mut c_void, value: &str, mut rect: RECT, flags: u32) {
    let value = wide(value);
    unsafe { DrawTextW(dc, value.as_ptr(), -1, &mut rect, flags) };
}

fn client_point(lparam: LPARAM) -> (i32, i32) {
    (signed_low_word(lparam), signed_high_word(lparam))
}

fn signed_low_word(value: LPARAM) -> i32 {
    (value as u16 as i16) as i32
}

fn signed_high_word(value: LPARAM) -> i32 {
    ((value >> 16) as u16 as i16) as i32
}

unsafe fn show_report_window(
    owner: HWND,
    title: &str,
    report: String,
) -> Result<(), std::io::Error> {
    let instance = unsafe { GetModuleHandleW(null()) };
    let class_name = wide(HARDWARE_CLASS_NAME);
    let class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(hardware_window_proc),
        hInstance: instance,
        hCursor: unsafe { LoadCursorW(null_mut(), IDC_ARROW) },
        hbrBackground: unsafe { GetStockObject(WHITE_BRUSH) as HBRUSH },
        lpszClassName: class_name.as_ptr(),
        ..unsafe { zeroed() }
    };
    unsafe { RegisterClassW(&class) };

    let state = Box::into_raw(Box::new(HardwareWindowState {
        report,
        edit: null_mut(),
        font: null_mut(),
    }));
    let title = wide(title);
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_APPWINDOW | WS_EX_CONTROLPARENT,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            920,
            720,
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
        UpdateWindow(hwnd);
    }
    Ok(())
}

unsafe extern "system" fn hardware_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        let create = lparam as *const CREATESTRUCTW;
        let state = unsafe { (*create).lpCreateParams as *mut HardwareWindowState };
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize) };
    }
    let state = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut HardwareWindowState };
    match message {
        WM_CREATE => {
            if state.is_null() {
                return -1;
            }
            let instance = unsafe { GetModuleHandleW(null()) };
            let edit = unsafe {
                child_ex(
                    hwnd,
                    instance,
                    "EDIT",
                    &(*state).report,
                    WS_EX_CLIENTEDGE,
                    WS_TABSTOP
                        | WS_HSCROLL
                        | WS_VSCROLL
                        | ES_MULTILINE
                        | ES_AUTOVSCROLL
                        | ES_AUTOHSCROLL
                        | ES_READONLY,
                    0,
                )
            };
            let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96);
            unsafe {
                (*state).edit = edit;
                (*state).font = create_ui_font_px((14 * dpi / 96) as i32, FW_NORMAL as i32);
                SendMessageW(edit, WM_SETFONT, (*state).font as WPARAM, 1);
                let mut rect: RECT = zeroed();
                GetClientRect(hwnd, &mut rect);
                move_window(edit, 8, 8, rect.right - 16, rect.bottom - 16);
            }
            0
        }
        WM_SIZE => {
            if !state.is_null() {
                let mut rect: RECT = unsafe { zeroed() };
                unsafe {
                    GetClientRect(hwnd, &mut rect);
                    move_window((*state).edit, 8, 8, rect.right - 16, rect.bottom - 16);
                }
            }
            0
        }
        WM_NCDESTROY => {
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };
            if !state.is_null() {
                let state = unsafe { Box::from_raw(state) };
                if !state.font.is_null() {
                    unsafe { DeleteObject(state.font) };
                }
            }
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn locate_package_root() -> Result<PathBuf, std::io::Error> {
    if let Some(root) = std::env::args_os().nth(1).map(PathBuf::from)
        && is_package_root(&root)
    {
        return Ok(root);
    }
    let current = std::env::current_dir()?;
    if is_package_root(&current) {
        return Ok(current);
    }
    let executable = std::env::current_exe()?;
    for ancestor in executable.ancestors().skip(1) {
        if is_package_root(ancestor) {
            return Ok(ancestor.to_path_buf());
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "找不到包含 List、tools 和 skin 的工具箱目录",
    ))
}

fn is_package_root(path: &Path) -> bool {
    path.join("List").is_dir() && path.join("tools").is_dir() && path.join("skin").is_dir()
}

pub fn show_fatal_error(message: &str) {
    unsafe {
        message_box(
            null_mut(),
            message,
            "图吧工具箱启动失败",
            MB_OK | MB_ICONERROR,
        )
    };
}

unsafe fn child_ex(
    parent: HWND,
    instance: HINSTANCE,
    class_name: &str,
    text: &str,
    extended_style: u32,
    style: u32,
    id: usize,
) -> HWND {
    let class_name = wide(class_name);
    let text = wide(text);
    unsafe {
        CreateWindowExW(
            extended_style,
            class_name.as_ptr(),
            text.as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            0,
            0,
            0,
            0,
            parent,
            id as HMENU,
            instance,
            null_mut(),
        )
    }
}

unsafe fn create_ui_font_px(pixel_height: i32, weight: i32) -> HGDIOBJ {
    let face = wide("Microsoft YaHei UI");
    unsafe {
        CreateFontW(
            -pixel_height.max(9),
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            DEFAULT_CHARSET.into(),
            OUT_DEFAULT_PRECIS.into(),
            CLIP_DEFAULT_PRECIS.into(),
            5,
            (DEFAULT_PITCH | FF_DONTCARE).into(),
            face.as_ptr(),
        ) as HGDIOBJ
    }
}

unsafe fn move_window(hwnd: HWND, x: i32, y: i32, width: i32, height: i32) {
    unsafe { MoveWindow(hwnd, x, y, width.max(0), height.max(0), 1) };
}

unsafe fn message_box(hwnd: HWND, message: &str, title: &str, style: u32) -> i32 {
    let message = wide(message);
    let title = wide(title);
    unsafe { MessageBoxW(hwnd, message.as_ptr(), title.as_ptr(), style) }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

unsafe fn open_skin_file(owner: HWND, initial_directory: &Path) -> Option<PathBuf> {
    let filter = wide("图吧工具箱皮肤 (*.skin)\0*.skin\0所有文件 (*.*)\0*.*\0");
    let title = wide("选择皮肤");
    let initial_directory = wide(&initial_directory.to_string_lossy());
    let extension = wide("skin");
    let mut file_name = vec![0u16; 32_768];
    let mut dialog: OPENFILENAMEW = unsafe { zeroed() };
    dialog.lStructSize = size_of::<OPENFILENAMEW>() as u32;
    dialog.hwndOwner = owner;
    dialog.lpstrFilter = filter.as_ptr();
    dialog.nFilterIndex = 1;
    dialog.lpstrFile = file_name.as_mut_ptr();
    dialog.nMaxFile = file_name.len() as u32;
    dialog.lpstrInitialDir = initial_directory.as_ptr();
    dialog.lpstrTitle = title.as_ptr();
    dialog.Flags = OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
    dialog.lpstrDefExt = extension.as_ptr();

    if unsafe { GetOpenFileNameW(&mut dialog) } == 0 {
        return None;
    }
    let length = file_name
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(file_name.len());
    Some(PathBuf::from(String::from_utf16_lossy(
        &file_name[..length],
    )))
}

const fn rgb(red: u8, green: u8, blue: u8) -> COLORREF {
    red as u32 | ((green as u32) << 8) | ((blue as u32) << 16)
}
