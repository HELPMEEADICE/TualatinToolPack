#![cfg(windows)]

use tbtool_core::{HardwareCategory, collect_hardware_snapshot};

#[test]
fn inventories_current_windows_machine() {
    let snapshot = collect_hardware_snapshot().unwrap();
    assert!(!snapshot.computer_name.is_empty());
    assert!(snapshot.device_count() >= 5, "{snapshot:#?}");
    for category in [
        HardwareCategory::System,
        HardwareCategory::Processor,
        HardwareCategory::Mainboard,
        HardwareCategory::Memory,
        HardwareCategory::Graphics,
        HardwareCategory::Storage,
        HardwareCategory::Network,
    ] {
        assert!(
            snapshot
                .sections
                .iter()
                .any(|section| section.category == category && !section.devices.is_empty()),
            "missing {category:?}: {snapshot:#?}"
        );
    }
    let report = snapshot.to_text();
    assert!(report.contains("【处理器】"));
    assert!(report.contains("【物理磁盘】"));
}
