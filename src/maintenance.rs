use crate::{Error, Result};
use serde_json::Value;
#[cfg(windows)]
use serde_json::json;

/// The updater runs outside this process so Windows can replace the executable.
pub fn update(check: bool, local: bool) -> Result<Value> {
    if check && local {
        return Err(Error::invalid("--check 与 --local 不能同时使用"));
    }
    #[cfg(not(windows))]
    {
        Err(Error::new(
            "update_unsupported",
            "当前安装更新工具只支持 Windows",
        ))
    }
    #[cfg(windows)]
    {
        use std::{
            fs::File,
            os::windows::process::CommandExt,
            process::{Command, Stdio},
        };
        let executable = std::env::current_exe()?;
        let directory = executable
            .parent()
            .ok_or_else(|| Error::new("not_installed", "无法定位程序目录"))?;
        let script = directory.join("update.ps1");
        if !script.is_file() || !directory.join("installation.json").is_file() {
            return Err(Error::new(
                "not_installed",
                "更新需使用已安装版本，请先运行 scripts/install.ps1",
            ));
        }
        let powershell = std::path::PathBuf::from(
            std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()),
        )
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let mut command = Command::new(powershell);
        // Do not inherit PowerShell 7's module paths into Windows PowerShell 5.1.
        command.env_remove("PSModulePath");
        command
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(script);
        command.creation_flags(0x08000000);
        if check {
            let output = command.arg("-CheckOnly").output()?;
            if !output.status.success() {
                return Err(Error::new(
                    "update_failed",
                    String::from_utf8_lossy(&output.stderr).trim().to_owned(),
                ));
            }
            return serde_json::from_slice(&output.stdout).map_err(Error::from);
        }
        if local {
            command.arg("-Local");
        }
        let log_path = directory.join("update.log");
        let log = File::create(&log_path)?;
        command
            .arg("-WaitForProcessId")
            .arg(std::process::id().to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log));
        let child = command.spawn()?;
        Ok(
            json!({"status":"started","updater_pid":child.id(),"log":log_path,"result_file":directory.join("update-status.json"),"mode":if local {"local"} else {"remote"}}),
        )
    }
}
