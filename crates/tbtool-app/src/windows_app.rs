use std::{
    ffi::c_void,
    mem::{size_of, zeroed},
    path::{Path, PathBuf},
    ptr::{null, null_mut},
};

use tbtool_core::{ToolCatalog, ToolLauncher, ToolTarget, collect_hardware_snapshot};
use windows_sys::Win32::{
    Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{
        BeginPaint, CLIP_DEFAULT_PRECIS, CreateFontW, CreateSolidBrush, DEFAULT_CHARSET,
        DEFAULT_PITCH, DeleteObject, EndPaint, FF_DONTCARE, FW_NORMAL, FillRect, GetStockObject,
        HBRUSH, HGDIOBJ, OUT_DEFAULT_PRECIS, PAINTSTRUCT, SetBkColor, SetBkMode, SetTextColor,
        TRANSPARENT, UpdateWindow, WHITE_BRUSH,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Controls::{ICC_LISTVIEW_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx},
        HiDpi::{
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow,
            SetProcessDpiAwarenessContext,
        },
        Input::KeyboardAndMouse::{EnableWindow, SetFocus},
        WindowsAndMessaging::{
            CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW,
            DispatchMessageW, EN_CHANGE, GWLP_USERDATA, GetClientRect, GetMessageW,
            GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, HMENU, IDC_ARROW,
            LBN_SELCHANGE, LoadCursorW, MB_DEFBUTTON2, MB_ICONERROR, MB_ICONWARNING, MB_OK,
            MB_YESNO, MSG, MessageBoxW, MoveWindow, PostQuitMessage, RegisterClassW, SW_SHOW,
            SendMessageW, SetWindowLongPtrW, SetWindowTextW, ShowWindow, TranslateMessage,
            WM_COMMAND, WM_CREATE, WM_CTLCOLORSTATIC, WM_DESTROY, WM_DPICHANGED, WM_ERASEBKGND,
            WM_GETMINMAXINFO, WM_NCCREATE, WM_NCDESTROY, WM_NOTIFY, WM_PAINT, WM_SETFONT, WM_SIZE,
            WNDCLASSW, WS_BORDER, WS_CHILD, WS_CLIPCHILDREN, WS_EX_APPWINDOW, WS_EX_CLIENTEDGE,
            WS_EX_CONTROLPARENT, WS_HSCROLL, WS_OVERLAPPEDWINDOW, WS_TABSTOP, WS_VISIBLE,
            WS_VSCROLL,
        },
    },
};

const CLASS_NAME: &str = "TbToolRustMainWindow";
const HARDWARE_CLASS_NAME: &str = "TbToolRustHardwareWindow";
const WINDOW_TITLE: &str = "图吧工具箱 2026 · Rust";
const PASSWORD: &[u8] = b"tulading123";

const ID_CATEGORY: usize = 1001;
const ID_SEARCH: usize = 1002;
const ID_TOOLS: usize = 1003;
const ID_LAUNCH: usize = 1004;
const ID_HARDWARE: usize = 1005;

const LBS_NOTIFY: u32 = 0x0001;
const LBS_NOINTEGRALHEIGHT: u32 = 0x0100;
const ES_AUTOHSCROLL: u32 = 0x0080;
const ES_MULTILINE: u32 = 0x0004;
const ES_AUTOVSCROLL: u32 = 0x0040;
const ES_READONLY: u32 = 0x0800;
const BS_PUSHBUTTON: u32 = 0;
const SS_LEFT: u32 = 0;
const SS_NOPREFIX: u32 = 0x0080;
const LVS_REPORT: u32 = 0x0001;
const LVS_SINGLESEL: u32 = 0x0004;
const LVS_SHOWSELALWAYS: u32 = 0x0008;
const LVS_NOSORTHEADER: u32 = 0x8000;
const LVS_EX_FULLROWSELECT: usize = 0x0020;
const LVS_EX_DOUBLEBUFFER: usize = 0x0001_0000;
const LVM_FIRST: u32 = 0x1000;
const LVM_DELETEALLITEMS: u32 = LVM_FIRST + 9;
const LVM_GETNEXTITEM: u32 = LVM_FIRST + 12;
const LVM_SETEXTENDEDLISTVIEWSTYLE: u32 = LVM_FIRST + 54;
const LVM_INSERTCOLUMNW: u32 = LVM_FIRST + 97;
const LVM_INSERTITEMW: u32 = LVM_FIRST + 77;
const LVM_SETITEMTEXTW: u32 = LVM_FIRST + 116;
const LVNI_SELECTED: usize = 0x0002;
const LVCF_FMT: u32 = 0x0001;
const LVCF_WIDTH: u32 = 0x0002;
const LVCF_TEXT: u32 = 0x0004;
const LVCFMT_LEFT: i32 = 0;
const LVIF_TEXT: u32 = 0x0001;
const LVIF_PARAM: u32 = 0x0004;
const LVIS_SELECTED: u32 = 0x0002;
const NM_DBLCLK: i32 = -3;
const NM_RETURN: i32 = -4;
const LVN_FIRST: i32 = -100;
const LVN_ITEMCHANGED: i32 = LVN_FIRST - 1;
const LB_ADDSTRING: u32 = 0x0180;
const LB_GETCURSEL: u32 = 0x0188;
const LB_SETCURSEL: u32 = 0x0186;
const EM_SETCUEBANNER: u32 = 0x1501;
const IDYES: i32 = 6;

#[repr(C)]
struct LvColumnW {
    mask: u32,
    fmt: i32,
    cx: i32,
    psz_text: *mut u16,
    cch_text_max: i32,
    i_sub_item: i32,
    i_image: i32,
    i_order: i32,
    cx_min: i32,
    cx_default: i32,
    cx_ideal: i32,
}

#[repr(C)]
struct LvItemW {
    mask: u32,
    i_item: i32,
    i_sub_item: i32,
    state: u32,
    state_mask: u32,
    psz_text: *mut u16,
    cch_text_max: i32,
    i_image: i32,
    l_param: LPARAM,
    i_indent: i32,
    i_group_id: i32,
    c_columns: u32,
    pu_columns: *mut u32,
    pi_col_fmt: *mut i32,
    i_group: i32,
}

#[repr(C)]
struct NmHdr {
    hwnd_from: HWND,
    id_from: usize,
    code: i32,
}

#[repr(C)]
struct NmListView {
    hdr: NmHdr,
    i_item: i32,
    i_sub_item: i32,
    u_new_state: u32,
    u_old_state: u32,
    u_changed: u32,
    pt_action: windows_sys::Win32::Foundation::POINT,
    l_param: LPARAM,
}

struct AppState {
    catalog: ToolCatalog,
    launcher: ToolLauncher,
    hwnd: HWND,
    header_title: HWND,
    header_subtitle: HWND,
    category_label: HWND,
    category: HWND,
    search_label: HWND,
    search: HWND,
    tools: HWND,
    detail_name: HWND,
    detail_description: HWND,
    status: HWND,
    launch: HWND,
    hardware: HWND,
    font: HGDIOBJ,
    title_font: HGDIOBJ,
    background_brush: HBRUSH,
    header_brush: HBRUSH,
    detail_brush: HBRUSH,
    active_category: usize,
    visible_tools: Vec<usize>,
}

struct HardwareWindowState {
    report: String,
    edit: HWND,
    font: HGDIOBJ,
}

impl AppState {
    fn new(catalog: ToolCatalog, launcher: ToolLauncher) -> Self {
        Self {
            catalog,
            launcher,
            hwnd: null_mut(),
            header_title: null_mut(),
            header_subtitle: null_mut(),
            category_label: null_mut(),
            category: null_mut(),
            search_label: null_mut(),
            search: null_mut(),
            tools: null_mut(),
            detail_name: null_mut(),
            detail_description: null_mut(),
            status: null_mut(),
            launch: null_mut(),
            hardware: null_mut(),
            font: null_mut(),
            title_font: null_mut(),
            background_brush: null_mut(),
            header_brush: null_mut(),
            detail_brush: null_mut(),
            active_category: 0,
            visible_tools: Vec::new(),
        }
    }

    unsafe fn create_controls(&mut self, instance: HINSTANCE) {
        let dpi = unsafe { GetDpiForWindow(self.hwnd) }.max(96);
        self.font = unsafe { create_ui_font(dpi, 14, FW_NORMAL as i32) };
        self.title_font = unsafe { create_ui_font(dpi, 22, 600) };
        self.background_brush = unsafe { CreateSolidBrush(rgb(245, 247, 246)) };
        self.header_brush = unsafe { CreateSolidBrush(rgb(27, 48, 43)) };
        self.detail_brush = unsafe { CreateSolidBrush(rgb(235, 241, 238)) };

        self.header_title =
            unsafe { child(self.hwnd, instance, "STATIC", WINDOW_TITLE, SS_NOPREFIX, 0) };
        self.header_subtitle = unsafe {
            child(
                self.hwnd,
                instance,
                "STATIC",
                "硬件识别 · 稳定性验证 · 驱动与维护",
                SS_NOPREFIX,
                0,
            )
        };
        self.category_label =
            unsafe { child(self.hwnd, instance, "STATIC", "工具分类", SS_NOPREFIX, 0) };
        self.category = unsafe {
            child(
                self.hwnd,
                instance,
                "LISTBOX",
                "",
                WS_BORDER | WS_VSCROLL | LBS_NOTIFY | LBS_NOINTEGRALHEIGHT,
                ID_CATEGORY,
            )
        };
        self.search_label =
            unsafe { child(self.hwnd, instance, "STATIC", "搜索工具", SS_NOPREFIX, 0) };
        self.search = unsafe {
            child_ex(
                self.hwnd,
                instance,
                "EDIT",
                "",
                WS_EX_CLIENTEDGE,
                WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL,
                ID_SEARCH,
            )
        };
        self.tools = unsafe {
            child_ex(
                self.hwnd,
                instance,
                "SysListView32",
                "",
                WS_EX_CLIENTEDGE,
                WS_BORDER
                    | WS_TABSTOP
                    | WS_VSCROLL
                    | LVS_REPORT
                    | LVS_SINGLESEL
                    | LVS_SHOWSELALWAYS
                    | LVS_NOSORTHEADER,
                ID_TOOLS,
            )
        };
        self.detail_name = unsafe {
            child(
                self.hwnd,
                instance,
                "STATIC",
                "选择一个工具",
                SS_NOPREFIX,
                0,
            )
        };
        self.detail_description = unsafe {
            child(
                self.hwnd,
                instance,
                "STATIC",
                "从列表中选择工具以查看说明和启动状态。",
                SS_LEFT | SS_NOPREFIX,
                0,
            )
        };
        self.status = unsafe {
            child(
                self.hwnd,
                instance,
                "STATIC",
                "正在载入工具目录…",
                SS_NOPREFIX,
                0,
            )
        };
        self.launch = unsafe {
            child(
                self.hwnd,
                instance,
                "BUTTON",
                "启动工具",
                WS_TABSTOP | BS_PUSHBUTTON,
                ID_LAUNCH,
            )
        };
        self.hardware = unsafe {
            child(
                self.hwnd,
                instance,
                "BUTTON",
                "硬件检测",
                WS_TABSTOP | BS_PUSHBUTTON,
                ID_HARDWARE,
            )
        };

        for hwnd in self.controls() {
            unsafe { SendMessageW(hwnd, WM_SETFONT, self.font as WPARAM, 1) };
        }
        unsafe { SendMessageW(self.header_title, WM_SETFONT, self.title_font as WPARAM, 1) };
        let cue = wide("搜索工具名称、说明或可执行文件");
        unsafe { SendMessageW(self.search, EM_SETCUEBANNER, 1, cue.as_ptr() as LPARAM) };
        unsafe {
            SendMessageW(
                self.tools,
                LVM_SETEXTENDEDLISTVIEWSTYLE,
                0,
                (LVS_EX_FULLROWSELECT | LVS_EX_DOUBLEBUFFER) as LPARAM,
            )
        };
        unsafe { insert_column(self.tools, 0, "工具", 230) };
        unsafe { insert_column(self.tools, 1, "用途与说明", 530) };

        for category in &self.catalog.categories {
            let text = wide(&category.name);
            unsafe { SendMessageW(self.category, LB_ADDSTRING, 0, text.as_ptr() as LPARAM) };
        }
        unsafe { SendMessageW(self.category, LB_SETCURSEL, 0, 0) };
        unsafe { EnableWindow(self.launch, 0) };
        unsafe {
            self.refresh_tools();
            self.layout();
        }
    }

    fn controls(&self) -> [HWND; 12] {
        [
            self.header_title,
            self.header_subtitle,
            self.category_label,
            self.category,
            self.search_label,
            self.search,
            self.tools,
            self.detail_name,
            self.detail_description,
            self.status,
            self.launch,
            self.hardware,
        ]
    }

    unsafe fn layout(&self) {
        let mut rect: RECT = unsafe { zeroed() };
        unsafe { GetClientRect(self.hwnd, &mut rect) };
        let width = rect.right.max(760);
        let height = rect.bottom.max(520);
        let dpi = unsafe { GetDpiForWindow(self.hwnd) }.max(96) as i32;
        let s = |value: i32| value * dpi / 96;
        let header = s(76);
        let sidebar = s(206);
        let gap = s(16);
        let right = s(18);
        let content_x = sidebar + gap;
        let content_w = width - content_x - right;
        let detail_h = s(112);
        let list_y = header + s(58);
        let list_h = height - list_y - detail_h - s(34);

        unsafe {
            move_window(self.header_title, s(20), s(12), width - s(280), s(32));
            move_window(self.header_subtitle, s(22), s(46), width - s(300), s(22));
            move_window(self.hardware, width - s(126), s(20), s(106), s(34));
            move_window(
                self.category_label,
                s(16),
                header + s(16),
                sidebar - s(28),
                s(22),
            );
            move_window(
                self.category,
                s(14),
                header + s(44),
                sidebar - s(26),
                height - header - s(60),
            );
            move_window(self.search_label, content_x, header + s(21), s(72), s(24));
            move_window(
                self.search,
                content_x + s(76),
                header + s(16),
                content_w - s(76),
                s(32),
            );
            move_window(self.tools, content_x, list_y, content_w, list_h);
            move_window(
                self.detail_name,
                content_x + s(12),
                list_y + list_h + s(10),
                content_w - s(144),
                s(24),
            );
            move_window(
                self.detail_description,
                content_x + s(12),
                list_y + list_h + s(36),
                content_w - s(154),
                s(48),
            );
            move_window(
                self.launch,
                content_x + content_w - s(122),
                list_y + list_h + s(24),
                s(110),
                s(36),
            );
            move_window(
                self.status,
                content_x + s(12),
                height - s(28),
                content_w - s(12),
                s(20),
            );
        }
    }

    unsafe fn refresh_tools(&mut self) {
        unsafe { SendMessageW(self.tools, LVM_DELETEALLITEMS, 0, 0) };
        self.visible_tools.clear();
        let query = unsafe { window_text(self.search) }.trim().to_lowercase();
        let Some(category) = self.catalog.categories.get(self.active_category) else {
            return;
        };
        for (tool_index, tool) in category.tools.iter().enumerate() {
            let matches = query.is_empty()
                || tool.name.to_lowercase().contains(&query)
                || tool.normalized_name.to_lowercase().contains(&query)
                || tool.description.to_lowercase().contains(&query)
                || tool.executable_text.to_lowercase().contains(&query);
            if !matches {
                continue;
            }
            let row = self.visible_tools.len() as i32;
            self.visible_tools.push(tool_index);
            unsafe { insert_item(self.tools, row, &tool.name, tool_index as LPARAM) };
            unsafe { set_item_text(self.tools, row, 1, &tool.description) };
        }
        let status = format!(
            "{} · 显示 {} / {} 个工具",
            category.name,
            self.visible_tools.len(),
            category.tools.len()
        );
        unsafe { set_text(self.status, &status) };
        unsafe { set_text(self.detail_name, "选择一个工具") };
        unsafe {
            set_text(
                self.detail_description,
                "从列表中选择工具以查看说明和启动状态。",
            )
        };
        unsafe { EnableWindow(self.launch, 0) };
    }

    unsafe fn set_category_from_control(&mut self) {
        let index = unsafe { SendMessageW(self.category, LB_GETCURSEL, 0, 0) } as isize;
        if index >= 0 {
            self.active_category = index as usize;
            unsafe { self.refresh_tools() };
        }
    }

    unsafe fn selected_tool(&self) -> Option<&tbtool_core::ToolEntry> {
        let row = unsafe {
            SendMessageW(
                self.tools,
                LVM_GETNEXTITEM,
                usize::MAX,
                LVNI_SELECTED as LPARAM,
            )
        } as isize;
        if row < 0 {
            return None;
        }
        let tool_index = *self.visible_tools.get(row as usize)?;
        self.catalog
            .categories
            .get(self.active_category)?
            .tools
            .get(tool_index)
    }

    unsafe fn selection_changed(&self) {
        let Some(tool) = (unsafe { self.selected_tool() }) else {
            return;
        };
        unsafe { set_text(self.detail_name, &tool.name) };
        let (availability, launchable) = match &tool.target {
            ToolTarget::BuiltIn { .. } => ("内置功能".to_owned(), true),
            target => match self.launcher.plan(target) {
                Ok(plan) => (format!("可启动 · {}", plan.executable.display()), true),
                Err(error) => (format!("不可启动 · {error}"), false),
            },
        };
        let description = if tool.description.trim().is_empty() {
            availability
        } else {
            format!("{}\r\n{}", tool.description.trim(), availability)
        };
        unsafe { set_text(self.detail_description, &description) };
        unsafe { EnableWindow(self.launch, launchable.into()) };
    }

    unsafe fn activate_selected(&mut self) {
        let Some(tool) = (unsafe { self.selected_tool() }) else {
            unsafe { SetFocus(self.tools) };
            return;
        };
        let name = tool.name.clone();
        match &tool.target {
            ToolTarget::BuiltIn { name: action } => {
                let action = action.clone();
                unsafe { self.run_builtin(&action) };
            }
            target => match self.launcher.launch(target) {
                Ok(launched) => unsafe {
                    let status = launched.process_id.map_or_else(
                        || format!("已通过系统关联打开 {name}"),
                        |process_id| format!("已启动 {name} · PID {process_id}"),
                    );
                    set_text(self.status, &status)
                },
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
    }

    unsafe fn show_hardware(&self) {
        unsafe { set_text(self.status, "正在读取 Windows 硬件清单…") };
        match collect_hardware_snapshot() {
            Ok(snapshot) => {
                let status = format!(
                    "硬件检测完成 · {} 个分区 · {} 个设备 · {} 条提示",
                    snapshot.sections.len(),
                    snapshot.device_count(),
                    snapshot.warnings.len()
                );
                unsafe { set_text(self.status, &status) };
                if let Err(error) = unsafe {
                    show_report_window(self.hwnd, "硬件检测报告", snapshot.to_text())
                } {
                    unsafe {
                        message_box(
                            self.hwnd,
                            &error.to_string(),
                            "硬件报告窗口创建失败",
                            MB_OK | MB_ICONERROR,
                        )
                    };
                }
            }
            Err(error) => unsafe {
                set_text(self.status, "硬件检测失败");
                message_box(
                    self.hwnd,
                    &error.to_string(),
                    "硬件检测失败",
                    MB_OK | MB_ICONERROR,
                );
            },
        }
    }

    unsafe fn run_builtin(&mut self, action: &str) {
        match action {
            "新手指引" => {
                let report = "图吧工具箱新手指引\r\n\r\n一、硬件信息\r\n使用主窗口右上角的“硬件检测”查看处理器、主板、内存、显卡、磁盘健康、显示器、网络、音频与电池信息。\r\n\r\n二、选择工具\r\n从左侧选择分类，在列表中查看用途说明。不可用或架构不兼容的工具会明确标记并禁止启动。\r\n\r\n三、稳定性测试\r\n烤机和压力测试会显著增加功耗与温度。开始前确认散热器、风扇和电源工作正常，并持续观察温度。\r\n\r\n四、驱动与固件\r\n更新 BIOS、显卡驱动或磁盘固件前先备份重要数据，只使用设备制造商提供的正式版本。\r\n";
                if let Err(error) =
                    unsafe { show_report_window(self.hwnd, "新手指引", report.to_owned()) }
                {
                    unsafe {
                        message_box(
                            self.hwnd,
                            &error.to_string(),
                            "无法打开新手指引",
                            MB_OK | MB_ICONERROR,
                        )
                    };
                }
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
                "即将同时启动 Prime95 CPU 压力测试和 FurMark 显卡压力测试。\r\n\r\n该操作会快速提高整机功耗与温度，请确认散热和电源状态正常，并持续监控温度。",
                "确认一键烤机",
                MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2,
            )
        };
        if choice != IDYES {
            unsafe { set_text(self.status, "已取消一键烤机") };
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
        unsafe { set_text(self.status, "已启动 Prime95 与 FurMark") };
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        unsafe {
            for object in [
                self.font,
                self.title_font,
                self.background_brush as HGDIOBJ,
                self.header_brush as HGDIOBJ,
                self.detail_brush as HGDIOBJ,
            ] {
                if !object.is_null() {
                    DeleteObject(object);
                }
            }
        }
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let root = locate_package_root()?;
    let catalog = ToolCatalog::load(&root, PASSWORD)?;
    let launcher = ToolLauncher::new(&root)?;
    let mut state = Box::new(AppState::new(catalog, launcher));

    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let controls = INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_LISTVIEW_CLASSES,
        };
        InitCommonControlsEx(&controls);
        let instance = GetModuleHandleW(null());
        let class_name = wide(CLASS_NAME);
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: GetStockObject(WHITE_BRUSH) as HBRUSH,
            lpszClassName: class_name.as_ptr(),
            ..zeroed()
        };
        if RegisterClassW(&class) == 0 {
            return Err(std::io::Error::last_os_error().into());
        }

        let title = wide(WINDOW_TITLE);
        let state_ptr = state.as_mut() as *mut AppState;
        let hwnd = CreateWindowExW(
            WS_EX_CONTROLPARENT,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1080,
            740,
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
        WM_CREATE => {
            if !state.is_null() {
                let instance = unsafe { GetModuleHandleW(null()) };
                unsafe { (*state).create_controls(instance) };
            }
            0
        }
        WM_SIZE => {
            if !state.is_null() {
                unsafe { (*state).layout() };
            }
            0
        }
        WM_DPICHANGED => {
            let suggested = lparam as *const RECT;
            if !suggested.is_null() {
                unsafe {
                    let rect = *suggested;
                    MoveWindow(
                        hwnd,
                        rect.left,
                        rect.top,
                        rect.right - rect.left,
                        rect.bottom - rect.top,
                        1,
                    );
                }
            }
            0
        }
        WM_COMMAND => {
            if !state.is_null() {
                let id = wparam & 0xffff;
                let notification = (wparam >> 16) as u32;
                unsafe {
                    match (id, notification) {
                        (ID_CATEGORY, value) if value == LBN_SELCHANGE => {
                            (*state).set_category_from_control()
                        }
                        (ID_SEARCH, value) if value == EN_CHANGE => (*state).refresh_tools(),
                        (ID_LAUNCH, _) => (*state).activate_selected(),
                        (ID_HARDWARE, _) => (*state).show_hardware(),
                        _ => {}
                    }
                }
            }
            0
        }
        WM_NOTIFY => {
            if !state.is_null() && lparam != 0 {
                let header = unsafe { &*(lparam as *const NmHdr) };
                if header.id_from == ID_TOOLS {
                    if header.code == NM_DBLCLK || header.code == NM_RETURN {
                        unsafe { (*state).activate_selected() };
                    } else if header.code == LVN_ITEMCHANGED {
                        let notification = unsafe { &*(lparam as *const NmListView) };
                        if notification.u_new_state & LVIS_SELECTED != 0 {
                            unsafe { (*state).selection_changed() };
                        }
                    }
                }
            }
            0
        }
        WM_CTLCOLORSTATIC => {
            if state.is_null() {
                return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
            }
            let dc = wparam as *mut c_void;
            let control = lparam as HWND;
            unsafe { SetBkMode(dc, TRANSPARENT as i32) };
            if control == unsafe { (*state).header_title }
                || control == unsafe { (*state).header_subtitle }
            {
                unsafe {
                    SetTextColor(dc, rgb(244, 250, 247));
                    SetBkColor(dc, rgb(27, 48, 43));
                    (*state).header_brush as LRESULT
                }
            } else if control == unsafe { (*state).category_label } {
                unsafe {
                    SetTextColor(dc, rgb(36, 57, 51));
                    SetBkColor(dc, rgb(235, 241, 238));
                    (*state).detail_brush as LRESULT
                }
            } else {
                unsafe {
                    SetTextColor(dc, rgb(36, 45, 42));
                    SetBkColor(dc, rgb(245, 247, 246));
                    (*state).background_brush as LRESULT
                }
            }
        }
        WM_GETMINMAXINFO => {
            if lparam != 0 {
                let limits = lparam as *mut windows_sys::Win32::UI::WindowsAndMessaging::MINMAXINFO;
                unsafe {
                    (*limits).ptMinTrackSize.x = 780;
                    (*limits).ptMinTrackSize.y = 560;
                }
            }
            0
        }
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            let mut paint: PAINTSTRUCT = unsafe { zeroed() };
            let dc = unsafe { BeginPaint(hwnd, &mut paint) };
            if !state.is_null() {
                let mut rect: RECT = unsafe { zeroed() };
                unsafe { GetClientRect(hwnd, &mut rect) };
                let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96) as i32;
                let header = 76 * dpi / 96;
                let sidebar = 206 * dpi / 96;
                unsafe {
                    FillRect(dc, &rect, (*state).background_brush);
                    let header_rect = RECT {
                        bottom: header,
                        ..rect
                    };
                    FillRect(dc, &header_rect, (*state).header_brush);
                    let sidebar_rect = RECT {
                        left: 0,
                        top: header,
                        right: sidebar,
                        bottom: rect.bottom,
                    };
                    FillRect(dc, &sidebar_rect, (*state).detail_brush);
                }
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

    let state = Box::new(HardwareWindowState {
        report,
        edit: null_mut(),
        font: null_mut(),
    });
    let state = Box::into_raw(state);
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
                (*state).font = create_ui_font(dpi, 11, FW_NORMAL as i32);
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
        WM_GETMINMAXINFO => {
            if lparam != 0 {
                let limits = lparam as *mut windows_sys::Win32::UI::WindowsAndMessaging::MINMAXINFO;
                unsafe {
                    (*limits).ptMinTrackSize.x = 600;
                    (*limits).ptMinTrackSize.y = 420;
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

unsafe fn child(
    parent: HWND,
    instance: HINSTANCE,
    class_name: &str,
    text: &str,
    style: u32,
    id: usize,
) -> HWND {
    unsafe { child_ex(parent, instance, class_name, text, 0, style, id) }
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

unsafe fn create_ui_font(dpi: u32, point_size: i32, weight: i32) -> HGDIOBJ {
    let face = wide("Microsoft YaHei UI");
    unsafe {
        CreateFontW(
            -(point_size * dpi as i32 / 72),
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

unsafe fn insert_column(hwnd: HWND, index: i32, text: &str, width: i32) {
    let mut text = wide(text);
    let mut column = LvColumnW {
        mask: LVCF_FMT | LVCF_WIDTH | LVCF_TEXT,
        fmt: LVCFMT_LEFT,
        cx: width,
        psz_text: text.as_mut_ptr(),
        cch_text_max: text.len() as i32,
        i_sub_item: index,
        i_image: 0,
        i_order: index,
        cx_min: 0,
        cx_default: 0,
        cx_ideal: 0,
    };
    unsafe {
        SendMessageW(
            hwnd,
            LVM_INSERTCOLUMNW,
            index as WPARAM,
            (&mut column as *mut LvColumnW) as LPARAM,
        )
    };
}

unsafe fn insert_item(hwnd: HWND, row: i32, text: &str, parameter: LPARAM) {
    let mut text = wide(text);
    let mut item: LvItemW = unsafe { zeroed() };
    item.mask = LVIF_TEXT | LVIF_PARAM;
    item.i_item = row;
    item.psz_text = text.as_mut_ptr();
    item.l_param = parameter;
    unsafe {
        SendMessageW(
            hwnd,
            LVM_INSERTITEMW,
            0,
            (&mut item as *mut LvItemW) as LPARAM,
        )
    };
}

unsafe fn set_item_text(hwnd: HWND, row: i32, column: i32, text: &str) {
    let mut text = wide(text);
    let mut item: LvItemW = unsafe { zeroed() };
    item.i_sub_item = column;
    item.psz_text = text.as_mut_ptr();
    unsafe {
        SendMessageW(
            hwnd,
            LVM_SETITEMTEXTW,
            row as WPARAM,
            (&mut item as *mut LvItemW) as LPARAM,
        )
    };
}

unsafe fn window_text(hwnd: HWND) -> String {
    let length = unsafe { GetWindowTextLengthW(hwnd) };
    let mut buffer = vec![0u16; length as usize + 1];
    unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    String::from_utf16_lossy(&buffer[..length as usize])
}

unsafe fn set_text(hwnd: HWND, text: &str) {
    let text = wide(text);
    unsafe { SetWindowTextW(hwnd, text.as_ptr()) };
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

const fn rgb(red: u8, green: u8, blue: u8) -> COLORREF {
    red as u32 | ((green as u32) << 8) | ((blue as u32) << 16)
}
