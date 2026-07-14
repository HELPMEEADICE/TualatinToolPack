#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareCategory {
    System,
    Processor,
    Mainboard,
    Memory,
    Graphics,
    Storage,
    Display,
    Network,
    Audio,
    Power,
    Usb,
    Thermal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareProperty {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareDevice {
    pub name: String,
    pub properties: Vec<HardwareProperty>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareSection {
    pub category: HardwareCategory,
    pub title: String,
    pub devices: Vec<HardwareDevice>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareSnapshot {
    pub computer_name: String,
    pub sections: Vec<HardwareSection>,
    pub warnings: Vec<String>,
}

impl HardwareSnapshot {
    pub fn device_count(&self) -> usize {
        self.sections
            .iter()
            .map(|section| section.devices.len())
            .sum()
    }

    pub fn to_text(&self) -> String {
        let mut text = format!("计算机：{}\r\n", self.computer_name);
        for section in &self.sections {
            text.push_str("\r\n");
            text.push('【');
            text.push_str(&section.title);
            text.push_str("】\r\n");
            if section.devices.is_empty() {
                text.push_str("  未检测到设备\r\n");
                continue;
            }
            for device in &section.devices {
                text.push_str("  ");
                text.push_str(&device.name);
                text.push_str("\r\n");
                for property in &device.properties {
                    text.push_str("    ");
                    text.push_str(&property.name);
                    text.push('：');
                    text.push_str(&property.value);
                    text.push_str("\r\n");
                }
            }
        }
        if !self.warnings.is_empty() {
            text.push_str("\r\n【检测提示】\r\n");
            for warning in &self.warnings {
                text.push_str("  ");
                text.push_str(warning);
                text.push_str("\r\n");
            }
        }
        text
    }
}

#[cfg(windows)]
mod platform {
    use std::collections::HashMap;

    use wmi::{COMLibrary, Variant, WMIConnection};

    use super::{
        HardwareCategory, HardwareDevice, HardwareProperty, HardwareSection, HardwareSnapshot,
    };
    use crate::{Error, Result};

    #[derive(Clone, Copy)]
    enum Format {
        Text,
        Bytes,
        Kibibytes,
        Megahertz,
        BitsPerSecond,
        Percent,
        CpuArchitecture,
        DriveType,
        BatteryStatus,
        Temperature,
        WmiDateTime,
        Utf16Array,
        StorageHealth,
        StorageMediaType,
        StorageBusType,
    }

    #[derive(Clone, Copy)]
    struct Field {
        source: &'static str,
        label: &'static str,
        format: Format,
    }

    struct Query {
        category: HardwareCategory,
        title: &'static str,
        class: &'static str,
        name: &'static str,
        fields: &'static [Field],
        filter: Option<&'static str>,
    }

    const SYSTEM_QUERIES: &[Query] = &[
        Query {
            category: HardwareCategory::System,
            title: "操作系统",
            class: "Win32_OperatingSystem",
            name: "Caption",
            fields: &[
                field("Version", "版本", Format::Text),
                field("BuildNumber", "内部版本", Format::Text),
                field("OSArchitecture", "系统架构", Format::Text),
                field("InstallDate", "安装时间", Format::WmiDateTime),
                field("LastBootUpTime", "最近启动", Format::WmiDateTime),
                field("TotalVisibleMemorySize", "可见内存", Format::Kibibytes),
                field("FreePhysicalMemory", "可用内存", Format::Kibibytes),
            ],
            filter: None,
        },
        Query {
            category: HardwareCategory::System,
            title: "整机信息",
            class: "Win32_ComputerSystem",
            name: "Model",
            fields: &[
                field("Manufacturer", "制造商", Format::Text),
                field("SystemType", "系统类型", Format::Text),
                field("TotalPhysicalMemory", "物理内存", Format::Bytes),
                field("Domain", "工作组/域", Format::Text),
            ],
            filter: None,
        },
        Query {
            category: HardwareCategory::Processor,
            title: "处理器",
            class: "Win32_Processor",
            name: "Name",
            fields: &[
                field("Manufacturer", "制造商", Format::Text),
                field("SocketDesignation", "插槽", Format::Text),
                field("NumberOfCores", "核心数", Format::Text),
                field("NumberOfLogicalProcessors", "逻辑处理器", Format::Text),
                field("MaxClockSpeed", "最高频率", Format::Megahertz),
                field("Architecture", "架构", Format::CpuArchitecture),
                field("AddressWidth", "地址宽度", Format::Text),
                field("ProcessorId", "处理器 ID", Format::Text),
                field("VirtualizationFirmwareEnabled", "固件虚拟化", Format::Text),
            ],
            filter: None,
        },
        Query {
            category: HardwareCategory::Mainboard,
            title: "主板",
            class: "Win32_BaseBoard",
            name: "Product",
            fields: &[
                field("Manufacturer", "制造商", Format::Text),
                field("Version", "版本", Format::Text),
                field("SerialNumber", "序列号", Format::Text),
            ],
            filter: None,
        },
        Query {
            category: HardwareCategory::Mainboard,
            title: "BIOS / UEFI",
            class: "Win32_BIOS",
            name: "SMBIOSBIOSVersion",
            fields: &[
                field("Manufacturer", "制造商", Format::Text),
                field("ReleaseDate", "发布日期", Format::WmiDateTime),
                field("SerialNumber", "序列号", Format::Text),
                field("SMBIOSMajorVersion", "SMBIOS 主版本", Format::Text),
                field("SMBIOSMinorVersion", "SMBIOS 次版本", Format::Text),
            ],
            filter: None,
        },
        Query {
            category: HardwareCategory::Memory,
            title: "物理内存条",
            class: "Win32_PhysicalMemory",
            name: "DeviceLocator",
            fields: &[
                field("BankLabel", "通道/插槽", Format::Text),
                field("Manufacturer", "制造商", Format::Text),
                field("PartNumber", "料号", Format::Text),
                field("SerialNumber", "序列号", Format::Text),
                field("Capacity", "容量", Format::Bytes),
                field("Speed", "标称频率", Format::Megahertz),
                field("ConfiguredClockSpeed", "当前频率", Format::Megahertz),
                field("ConfiguredVoltage", "当前电压 (mV)", Format::Text),
            ],
            filter: None,
        },
        Query {
            category: HardwareCategory::Graphics,
            title: "图形适配器",
            class: "Win32_VideoController",
            name: "Name",
            fields: &[
                field("VideoProcessor", "图形处理器", Format::Text),
                field("AdapterRAM", "显存", Format::Bytes),
                field("DriverVersion", "驱动版本", Format::Text),
                field("DriverDate", "驱动日期", Format::WmiDateTime),
                field("CurrentHorizontalResolution", "水平分辨率", Format::Text),
                field("CurrentVerticalResolution", "垂直分辨率", Format::Text),
                field("CurrentRefreshRate", "刷新率 (Hz)", Format::Text),
                field("Status", "状态", Format::Text),
                field("PNPDeviceID", "PnP 标识", Format::Text),
            ],
            filter: None,
        },
        Query {
            category: HardwareCategory::Storage,
            title: "物理磁盘",
            class: "Win32_DiskDrive",
            name: "Model",
            fields: &[
                field("SerialNumber", "序列号", Format::Text),
                field("FirmwareRevision", "固件", Format::Text),
                field("InterfaceType", "接口", Format::Text),
                field("MediaType", "介质", Format::Text),
                field("Size", "容量", Format::Bytes),
                field("Status", "状态", Format::Text),
                field("PNPDeviceID", "PnP 标识", Format::Text),
            ],
            filter: None,
        },
        Query {
            category: HardwareCategory::Storage,
            title: "逻辑磁盘",
            class: "Win32_LogicalDisk",
            name: "DeviceID",
            fields: &[
                field("VolumeName", "卷标", Format::Text),
                field("FileSystem", "文件系统", Format::Text),
                field("DriveType", "类型", Format::DriveType),
                field("Size", "容量", Format::Bytes),
                field("FreeSpace", "可用空间", Format::Bytes),
            ],
            filter: Some("DriveType >= 2"),
        },
        Query {
            category: HardwareCategory::Display,
            title: "显示器",
            class: "Win32_DesktopMonitor",
            name: "Name",
            fields: &[
                field("MonitorManufacturer", "制造商", Format::Text),
                field("ScreenWidth", "水平像素", Format::Text),
                field("ScreenHeight", "垂直像素", Format::Text),
                field("Status", "状态", Format::Text),
                field("PNPDeviceID", "PnP 标识", Format::Text),
            ],
            filter: None,
        },
        Query {
            category: HardwareCategory::Network,
            title: "物理网络适配器",
            class: "Win32_NetworkAdapter",
            name: "Name",
            fields: &[
                field("NetConnectionID", "连接名称", Format::Text),
                field("Manufacturer", "制造商", Format::Text),
                field("MACAddress", "MAC 地址", Format::Text),
                field("Speed", "链路速度", Format::BitsPerSecond),
                field("NetEnabled", "已启用", Format::Text),
                field("AdapterType", "类型", Format::Text),
                field("PNPDeviceID", "PnP 标识", Format::Text),
            ],
            filter: Some("PhysicalAdapter = TRUE"),
        },
        Query {
            category: HardwareCategory::Network,
            title: "网络地址配置",
            class: "Win32_NetworkAdapterConfiguration",
            name: "Description",
            fields: &[
                field("MACAddress", "MAC 地址", Format::Text),
                field("IPAddress", "IP 地址", Format::Text),
                field("IPSubnet", "子网掩码", Format::Text),
                field("DefaultIPGateway", "默认网关", Format::Text),
                field("DNSServerSearchOrder", "DNS 服务器", Format::Text),
                field("DHCPEnabled", "DHCP", Format::Text),
                field("DHCPServer", "DHCP 服务器", Format::Text),
            ],
            filter: Some("IPEnabled = TRUE"),
        },
        Query {
            category: HardwareCategory::Audio,
            title: "音频设备",
            class: "Win32_SoundDevice",
            name: "Name",
            fields: &[
                field("Manufacturer", "制造商", Format::Text),
                field("ProductName", "产品", Format::Text),
                field("Status", "状态", Format::Text),
                field("PNPDeviceID", "PnP 标识", Format::Text),
            ],
            filter: None,
        },
        Query {
            category: HardwareCategory::Power,
            title: "电池",
            class: "Win32_Battery",
            name: "Name",
            fields: &[
                field("DeviceID", "设备标识", Format::Text),
                field("BatteryStatus", "状态", Format::BatteryStatus),
                field("EstimatedChargeRemaining", "剩余电量", Format::Percent),
                field("EstimatedRunTime", "预计续航 (分钟)", Format::Text),
                field("DesignCapacity", "设计容量 (mWh)", Format::Text),
                field("FullChargeCapacity", "满充容量 (mWh)", Format::Text),
            ],
            filter: None,
        },
        Query {
            category: HardwareCategory::Usb,
            title: "USB 控制器",
            class: "Win32_USBController",
            name: "Name",
            fields: &[
                field("Manufacturer", "制造商", Format::Text),
                field("Status", "状态", Format::Text),
                field("PNPDeviceID", "PnP 标识", Format::Text),
            ],
            filter: None,
        },
        Query {
            category: HardwareCategory::System,
            title: "存在问题的 PnP 设备",
            class: "Win32_PnPEntity",
            name: "Name",
            fields: &[
                field("Manufacturer", "制造商", Format::Text),
                field("Status", "状态", Format::Text),
                field("ConfigManagerErrorCode", "设备管理器错误码", Format::Text),
                field("PNPDeviceID", "PnP 标识", Format::Text),
            ],
            filter: Some("ConfigManagerErrorCode <> 0"),
        },
    ];

    const THERMAL_QUERY: Query = Query {
        category: HardwareCategory::Thermal,
        title: "ACPI 温度区",
        class: "MSAcpi_ThermalZoneTemperature",
        name: "InstanceName",
        fields: &[
            field("CurrentTemperature", "当前温度", Format::Temperature),
            field("CriticalTripPoint", "临界温度", Format::Temperature),
            field("Active", "活动", Format::Text),
        ],
        filter: None,
    };

    const EDID_QUERY: Query = Query {
        category: HardwareCategory::Display,
        title: "显示器 EDID",
        class: "WmiMonitorID",
        name: "InstanceName",
        fields: &[
            field("ManufacturerName", "制造商代码", Format::Utf16Array),
            field("UserFriendlyName", "型号", Format::Utf16Array),
            field("SerialNumberID", "序列号", Format::Utf16Array),
            field("YearOfManufacture", "生产年份", Format::Text),
            field("WeekOfManufacture", "生产周", Format::Text),
            field("Active", "活动", Format::Text),
        ],
        filter: Some("Active = TRUE"),
    };

    const STORAGE_HEALTH_QUERY: Query = Query {
        category: HardwareCategory::Storage,
        title: "磁盘介质与健康状态",
        class: "MSFT_PhysicalDisk",
        name: "FriendlyName",
        fields: &[
            field("SerialNumber", "序列号", Format::Text),
            field("FirmwareVersion", "固件", Format::Text),
            field("Size", "容量", Format::Bytes),
            field("MediaType", "介质类型", Format::StorageMediaType),
            field("BusType", "总线", Format::StorageBusType),
            field("HealthStatus", "健康状态", Format::StorageHealth),
            field("OperationalStatus", "运行状态码", Format::Text),
            field("SpindleSpeed", "主轴转速 (RPM)", Format::Text),
        ],
        filter: None,
    };

    const fn field(source: &'static str, label: &'static str, format: Format) -> Field {
        Field {
            source,
            label,
            format,
        }
    }

    pub fn collect() -> Result<HardwareSnapshot> {
        let com = COMLibrary::new().map_err(wmi_error)?;
        let connection = WMIConnection::new(com).map_err(wmi_error)?;
        let mut snapshot = HardwareSnapshot {
            computer_name: std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Windows PC".into()),
            sections: Vec::with_capacity(SYSTEM_QUERIES.len() + 1),
            warnings: Vec::new(),
        };
        for query in SYSTEM_QUERIES {
            collect_query(&connection, query, &mut snapshot);
        }

        match WMIConnection::with_namespace_path("ROOT\\WMI", com) {
            Ok(root_wmi) => {
                collect_query(&root_wmi, &THERMAL_QUERY, &mut snapshot);
                collect_query(&root_wmi, &EDID_QUERY, &mut snapshot);
            }
            Err(error) => snapshot.warnings.push(format!("ACPI / EDID：{error}")),
        }
        match WMIConnection::with_namespace_path("ROOT\\Microsoft\\Windows\\Storage", com) {
            Ok(storage) => collect_query(&storage, &STORAGE_HEALTH_QUERY, &mut snapshot),
            Err(error) => snapshot
                .warnings
                .push(format!("磁盘介质与健康状态：{error}")),
        }
        Ok(snapshot)
    }

    fn collect_query(connection: &WMIConnection, query: &Query, snapshot: &mut HardwareSnapshot) {
        let columns = std::iter::once(query.name)
            .chain(query.fields.iter().map(|field| field.source))
            .collect::<Vec<_>>()
            .join(", ");
        let mut statement = format!("SELECT {columns} FROM {}", query.class);
        if let Some(filter) = query.filter {
            statement.push_str(" WHERE ");
            statement.push_str(filter);
        }
        match connection.raw_query::<HashMap<String, Variant>>(&statement) {
            Ok(rows) => {
                let devices = rows
                    .into_iter()
                    .enumerate()
                    .map(|(index, row)| row_to_device(query, index, row))
                    .collect();
                snapshot.sections.push(HardwareSection {
                    category: query.category,
                    title: query.title.to_owned(),
                    devices,
                });
            }
            Err(error) => snapshot
                .warnings
                .push(format!("{}：{}", query.title, error)),
        }
    }

    fn row_to_device(
        query: &Query,
        index: usize,
        mut row: HashMap<String, Variant>,
    ) -> HardwareDevice {
        let name = row
            .remove(query.name)
            .and_then(|value| format_value(&value, Format::Text))
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| format!("{} #{}", query.title, index + 1));
        let properties = query
            .fields
            .iter()
            .filter_map(|field| {
                let value = format_value(row.get(field.source)?, field.format)?;
                Some(HardwareProperty {
                    name: field.label.to_owned(),
                    value,
                })
            })
            .collect();
        HardwareDevice { name, properties }
    }

    fn format_value(value: &Variant, format: Format) -> Option<String> {
        let number = variant_u64(value);
        let text = match format {
            Format::Bytes => format_bytes(number?),
            Format::Kibibytes => format_bytes(number?.saturating_mul(1024)),
            Format::Megahertz => format!("{} MHz", number?),
            Format::BitsPerSecond => format_rate(number?),
            Format::Percent => format!("{}%", number?),
            Format::CpuArchitecture => match number? {
                0 => "x86".into(),
                5 => "ARM".into(),
                6 => "IA-64".into(),
                9 => "x64".into(),
                12 => "ARM64".into(),
                value => format!("未知 ({value})"),
            },
            Format::DriveType => match number? {
                2 => "可移动磁盘".into(),
                3 => "本地磁盘".into(),
                4 => "网络磁盘".into(),
                5 => "光盘".into(),
                6 => "RAM 磁盘".into(),
                value => format!("未知 ({value})"),
            },
            Format::BatteryStatus => match number? {
                1 => "放电中".into(),
                2 => "接通电源".into(),
                3 => "已充满".into(),
                4 => "低电量".into(),
                5 => "严重低电量".into(),
                6 => "充电中".into(),
                value => format!("状态码 {value}"),
            },
            Format::Temperature => format!("{:.1} °C", number? as f64 / 10.0 - 273.15),
            Format::WmiDateTime => format_wmi_datetime(&variant_text(value)?)?,
            Format::Utf16Array => format_utf16_array(value)?,
            Format::StorageHealth => match number? {
                0 => "正常".into(),
                1 => "警告".into(),
                2 => "不健康".into(),
                5 => "未知".into(),
                value => format!("状态码 {value}"),
            },
            Format::StorageMediaType => match number? {
                0 => "未指定".into(),
                3 => "机械硬盘 (HDD)".into(),
                4 => "固态硬盘 (SSD)".into(),
                5 => "存储级内存 (SCM)".into(),
                value => format!("类型码 {value}"),
            },
            Format::StorageBusType => match number? {
                0 => "未知".into(),
                3 => "ATA".into(),
                7 => "USB".into(),
                8 => "RAID".into(),
                10 => "SAS".into(),
                11 => "SATA".into(),
                12 => "SD".into(),
                13 => "MMC".into(),
                17 => "NVMe".into(),
                18 => "SCM".into(),
                value => format!("总线码 {value}"),
            },
            Format::Text => variant_text(value)?,
        };
        (!text.trim().is_empty()).then_some(text)
    }

    fn variant_text(value: &Variant) -> Option<String> {
        match value {
            Variant::Empty | Variant::Null => None,
            Variant::String(value) => Some(value.trim().to_owned()),
            Variant::Bool(value) => Some(if *value { "是" } else { "否" }.into()),
            Variant::I1(value) => Some(value.to_string()),
            Variant::I2(value) => Some(value.to_string()),
            Variant::I4(value) => Some(value.to_string()),
            Variant::I8(value) => Some(value.to_string()),
            Variant::UI1(value) => Some(value.to_string()),
            Variant::UI2(value) => Some(value.to_string()),
            Variant::UI4(value) => Some(value.to_string()),
            Variant::UI8(value) => Some(value.to_string()),
            Variant::R4(value) => Some(value.to_string()),
            Variant::R8(value) => Some(value.to_string()),
            Variant::Array(values) => Some(
                values
                    .iter()
                    .filter_map(variant_text)
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            Variant::Unknown(_) | Variant::Object(_) => None,
        }
    }

    fn variant_u64(value: &Variant) -> Option<u64> {
        match value {
            Variant::I1(value) => (*value).try_into().ok(),
            Variant::I2(value) => (*value).try_into().ok(),
            Variant::I4(value) => (*value).try_into().ok(),
            Variant::I8(value) => (*value).try_into().ok(),
            Variant::UI1(value) => Some((*value).into()),
            Variant::UI2(value) => Some((*value).into()),
            Variant::UI4(value) => Some((*value).into()),
            Variant::UI8(value) => Some(*value),
            Variant::String(value) => value.trim().parse().ok(),
            _ => None,
        }
    }

    fn format_wmi_datetime(value: &str) -> Option<String> {
        let digits = value.get(..14)?;
        if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return Some(value.to_owned());
        }
        Some(format!(
            "{}-{}-{} {}:{}:{}",
            &digits[0..4],
            &digits[4..6],
            &digits[6..8],
            &digits[8..10],
            &digits[10..12],
            &digits[12..14]
        ))
    }

    fn format_utf16_array(value: &Variant) -> Option<String> {
        let Variant::Array(values) = value else {
            return variant_text(value);
        };
        let units: Vec<u16> = values
            .iter()
            .filter_map(variant_u64)
            .take_while(|value| *value != 0)
            .filter_map(|value| value.try_into().ok())
            .collect();
        (!units.is_empty()).then(|| String::from_utf16_lossy(&units))
    }

    fn format_bytes(value: u64) -> String {
        const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
        let mut scaled = value as f64;
        let mut unit = 0;
        while scaled >= 1024.0 && unit + 1 < UNITS.len() {
            scaled /= 1024.0;
            unit += 1;
        }
        if unit == 0 {
            format!("{value} B")
        } else {
            format!("{scaled:.2} {}", UNITS[unit])
        }
    }

    fn format_rate(value: u64) -> String {
        if value >= 1_000_000_000 {
            format!("{:.2} Gbit/s", value as f64 / 1_000_000_000.0)
        } else if value >= 1_000_000 {
            format!("{:.0} Mbit/s", value as f64 / 1_000_000.0)
        } else if value >= 1_000 {
            format!("{:.0} kbit/s", value as f64 / 1_000.0)
        } else {
            format!("{value} bit/s")
        }
    }

    fn wmi_error(error: wmi::WMIError) -> Error {
        Error::HardwareDetection(error.to_string())
    }
}

#[cfg(windows)]
pub fn collect_hardware_snapshot() -> crate::Result<HardwareSnapshot> {
    platform::collect()
}

#[cfg(not(windows))]
pub fn collect_hardware_snapshot() -> crate::Result<HardwareSnapshot> {
    Err(crate::Error::HardwareDetection(
        "hardware inventory is only available on Windows".into(),
    ))
}
