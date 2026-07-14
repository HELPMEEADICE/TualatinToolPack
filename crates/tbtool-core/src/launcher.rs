use std::{
    fmt,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::Command,
};

use crate::{Error, Result, ToolTarget};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchPlan {
    pub executable: PathBuf,
    pub working_directory: PathBuf,
    pub architecture: ExecutableArchitecture,
    pub method: LaunchMethod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchMethod {
    Direct,
    Script,
    ShellAssociation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaunchedTool {
    pub process_id: Option<u32>,
    pub method: LaunchMethod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutableArchitecture {
    X86,
    X64,
    Arm,
    Arm64,
    Script,
    Unknown,
}

impl fmt::Display for ExecutableArchitecture {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::X86 => "x86",
            Self::X64 => "x64",
            Self::Arm => "ARM",
            Self::Arm64 => "ARM64",
            Self::Script => "script",
            Self::Unknown => "unknown-format",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostArchitecture {
    X86,
    X64,
    Arm64,
    Other,
}

impl HostArchitecture {
    pub fn current() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            Self::X64
        }
        #[cfg(target_arch = "x86")]
        {
            if std::env::var_os("PROCESSOR_ARCHITEW6432")
                .is_some_and(|value| value.to_string_lossy().eq_ignore_ascii_case("AMD64"))
            {
                Self::X64
            } else {
                Self::X86
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            Self::Arm64
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "x86", target_arch = "aarch64")))]
        {
            Self::Other
        }
    }

    pub fn supports(self, executable: ExecutableArchitecture) -> bool {
        matches!(
            (self, executable),
            (
                _,
                ExecutableArchitecture::Script | ExecutableArchitecture::Unknown
            ) | (Self::X86, ExecutableArchitecture::X86)
                | (
                    Self::X64,
                    ExecutableArchitecture::X86 | ExecutableArchitecture::X64
                )
                | (
                    Self::Arm64,
                    ExecutableArchitecture::X86
                        | ExecutableArchitecture::X64
                        | ExecutableArchitecture::Arm
                        | ExecutableArchitecture::Arm64,
                )
        )
    }
}

impl fmt::Display for HostArchitecture {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::X86 => "x86",
            Self::X64 => "x64",
            Self::Arm64 => "ARM64",
            Self::Other => "other",
        })
    }
}

#[derive(Debug, Clone)]
pub struct ToolLauncher {
    root: PathBuf,
    tools_root: PathBuf,
    host_architecture: HostArchitecture,
}

impl ToolLauncher {
    pub fn new(root: &Path) -> Result<Self> {
        let root = canonicalize_existing(root)?;
        let tools_root = canonicalize_existing(&root.join("tools"))?;
        if !tools_root.starts_with(&root) {
            return Err(Error::ToolOutsideRoot(tools_root.display().to_string()));
        }
        Ok(Self {
            root,
            tools_root,
            host_architecture: HostArchitecture::current(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn host_architecture(&self) -> HostArchitecture {
        self.host_architecture
    }

    pub fn plan(&self, target: &ToolTarget) -> Result<LaunchPlan> {
        let ToolTarget::Executable {
            path,
            working_directory,
        } = target
        else {
            let ToolTarget::BuiltIn { name } = target else {
                unreachable!()
            };
            return Err(Error::BuiltInAction(name.clone()));
        };

        let (path, working_directory) = self.compatible_paths(path, working_directory);
        let executable = canonicalize_existing(&self.root.join(path))?;
        let working_directory = canonicalize_existing(&self.root.join(working_directory))?;
        for resolved in [&executable, &working_directory] {
            if !resolved.starts_with(&self.tools_root) {
                return Err(Error::ToolOutsideRoot(resolved.display().to_string()));
            }
        }
        if !executable.is_file() || !working_directory.is_dir() {
            return Err(Error::InvalidToolPath(executable.display().to_string()));
        }
        let architecture = detect_executable_architecture(&executable)?;
        if !self.host_architecture.supports(architecture) {
            return Err(Error::UnsupportedToolArchitecture {
                host: self.host_architecture,
                executable: architecture,
                path: executable.display().to_string(),
            });
        }
        let method = match architecture {
            ExecutableArchitecture::Script => LaunchMethod::Script,
            ExecutableArchitecture::Unknown => LaunchMethod::ShellAssociation,
            _ => LaunchMethod::Direct,
        };
        Ok(LaunchPlan {
            executable,
            working_directory,
            architecture,
            method,
        })
    }

    pub fn launch(&self, target: &ToolTarget) -> Result<LaunchedTool> {
        let plan = self.plan(target)?;
        if plan.method == LaunchMethod::ShellAssociation {
            shell_execute(&plan)
        } else {
            let child = Command::new(&plan.executable)
                .current_dir(&plan.working_directory)
                .spawn()?;
            Ok(LaunchedTool {
                process_id: Some(child.id()),
                method: plan.method,
            })
        }
    }

    fn compatible_paths<'a>(
        &'a self,
        path: &'a Path,
        working_directory: &'a Path,
    ) -> (&'a Path, &'a Path) {
        let directory_text = working_directory.to_string_lossy();
        let is_legacy_ddu = directory_text.trim_end().ends_with("DDU v18.0.1.9")
            && directory_text.contains("显卡工具");
        if is_legacy_ddu && !self.root.join(working_directory).exists() {
            (path, Path::new("tools/显卡工具/DDU"))
        } else {
            (path, working_directory)
        }
    }
}

fn canonicalize_existing(path: &Path) -> Result<PathBuf> {
    path.canonicalize()
        .map_err(|_| Error::ToolNotFound(path.display().to_string()))
}

pub fn detect_executable_architecture(path: &Path) -> Result<ExecutableArchitecture> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    if ["bat", "cmd", "ps1", "vbs", "js"]
        .iter()
        .any(|script| extension.eq_ignore_ascii_case(script))
    {
        return Ok(ExecutableArchitecture::Script);
    }

    let mut file = File::open(path)?;
    let mut dos_header = [0u8; 64];
    if file.read_exact(&mut dos_header).is_err() || &dos_header[..2] != b"MZ" {
        return Ok(ExecutableArchitecture::Unknown);
    }
    let pe_offset = u32::from_le_bytes(dos_header[0x3c..0x40].try_into().unwrap());
    file.seek(SeekFrom::Start(pe_offset.into()))?;
    let mut pe_header = [0u8; 6];
    if file.read_exact(&mut pe_header).is_err() || &pe_header[..4] != b"PE\0\0" {
        return Ok(ExecutableArchitecture::Unknown);
    }
    Ok(
        match u16::from_le_bytes(pe_header[4..6].try_into().unwrap()) {
            0x014c => ExecutableArchitecture::X86,
            0x8664 => ExecutableArchitecture::X64,
            0x01c0 | 0x01c2 | 0x01c4 => ExecutableArchitecture::Arm,
            0xaa64 => ExecutableArchitecture::Arm64,
            _ => ExecutableArchitecture::Unknown,
        },
    )
}

#[cfg(windows)]
fn shell_execute(plan: &LaunchPlan) -> Result<LaunchedTool> {
    use std::{
        os::windows::ffi::OsStrExt,
        ptr::{null, null_mut},
    };
    use windows_sys::Win32::{UI::Shell::ShellExecuteW, UI::WindowsAndMessaging::SW_SHOWNORMAL};

    let executable: Vec<u16> = plan
        .executable
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let directory: Vec<u16> = plan
        .working_directory
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            null(),
            executable.as_ptr(),
            null(),
            directory.as_ptr(),
            SW_SHOWNORMAL,
        )
    } as isize;
    if result <= 32 {
        return Err(std::io::Error::from_raw_os_error(result as i32).into());
    }
    Ok(LaunchedTool {
        process_id: None,
        method: plan.method,
    })
}

#[cfg(not(windows))]
fn shell_execute(plan: &LaunchPlan) -> Result<LaunchedTool> {
    Err(Error::InvalidToolPath(format!(
        "shell association is unavailable for {}",
        plan.executable.display()
    )))
}
