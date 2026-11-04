#![cfg(windows)]

use std::ffi::OsStr;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::process::Command;

use windows::Win32::Foundation::{CloseHandle, HANDLE, HLOCAL, LocalFree, WAIT_OBJECT_0};
use windows::Win32::Security::Authorization::{ConvertSidToStringSidW, ConvertStringSidToSidW};
use windows::Win32::Security::Isolation::{
    CreateAppContainerProfile, DeleteAppContainerProfile, DeriveAppContainerSidFromAppContainerName,
};
use windows::Win32::Security::{FreeSid, PSID, SECURITY_CAPABILITIES, SID_AND_ATTRIBUTES};
use windows::Win32::System::SystemServices::SE_GROUP_ENABLED;
use windows::Win32::System::Threading::{
    CreateProcessW, DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT,
    InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
    PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, PROCESS_INFORMATION, STARTUPINFOEXW,
    UpdateProcThreadAttribute,
};
use windows::core::{HSTRING, PCWSTR, PWSTR};

/// A per-user AppContainer profile. The SID is allocated by the profile API and
/// must be released with `FreeSid`.
pub struct AppContainerProfile {
    pub name: String,
    pub sid: PSID,
    created: bool,
}

impl AppContainerProfile {
    pub fn create(name: &str) -> Result<Self, String> {
        let hname = HSTRING::from(name);
        let mut profile_capability_sids = Vec::new();
        for sid_text in ["S-1-15-3-1", "S-1-15-3-2", "S-1-15-3-3"] {
            let mut sid = PSID::default();
            unsafe {
                ConvertStringSidToSidW(&HSTRING::from(sid_text), &mut sid).map_err(|error| {
                    format!("ConvertStringSidToSidW({sid_text}) for profile failed: {error}")
                })?;
            }
            profile_capability_sids.push(sid);
        }
        let mut profile_capabilities: Vec<SID_AND_ATTRIBUTES> = profile_capability_sids
            .iter()
            .map(|sid| SID_AND_ATTRIBUTES {
                Sid: *sid,
                Attributes: SE_GROUP_ENABLED as u32,
            })
            .collect();
        let result = unsafe {
            CreateAppContainerProfile(
                &hname,
                &hname,
                &HSTRING::from("Forge Windows proxy spike profile"),
                Some(&profile_capabilities),
            )
        };
        for sid in &profile_capability_sids {
            unsafe {
                LocalFree(Some(HLOCAL(sid.0)));
            }
        }
        profile_capabilities.clear();
        match result {
            Ok(sid) => Ok(Self {
                name: name.to_string(),
                sid,
                created: true,
            }),
            Err(error) => {
                // A previous crashed run may have left the profile behind.
                // Deriving the SID is enough to reuse it.
                let derived = unsafe { DeriveAppContainerSidFromAppContainerName(&hname) };
                match derived {
                    Ok(derived_sid) => Ok(Self {
                        name: name.to_string(),
                        sid: derived_sid,
                        created: false,
                    }),
                    Err(derive_error) => Err(format!(
                        "CreateAppContainerProfile failed ({error}); DeriveAppContainerSidFromAppContainerName also failed ({derive_error})"
                    )),
                }
            }
        }
    }

    pub fn sid_string(&self) -> Result<String, String> {
        unsafe {
            let mut buffer = PWSTR::null();
            ConvertSidToStringSidW(self.sid, &mut buffer)
                .map_err(|error| format!("ConvertSidToStringSidW failed: {error}"))?;
            let value = buffer
                .to_string()
                .map_err(|error| format!("SID string was not valid UTF-16: {error}"))?;
            LocalFree(Some(HLOCAL(buffer.0 as *mut core::ffi::c_void)));
            Ok(value)
        }
    }

    pub fn delete(&mut self) -> String {
        if !self.created {
            return "profile was pre-existing; not deleted".to_string();
        }
        let hname = HSTRING::from(&self.name);
        match unsafe { DeleteAppContainerProfile(&hname) } {
            Ok(()) => {
                self.created = false;
                "profile deleted".to_string()
            }
            Err(error) => format!("DeleteAppContainerProfile failed: {error}"),
        }
    }
}

impl Drop for AppContainerProfile {
    fn drop(&mut self) {
        unsafe {
            FreeSid(self.sid);
        }
    }
}

/// Grant the AppContainer SID access to the spike directory. This is a test
/// fixture grant, not a Forge deployment mechanism.
pub fn grant_access(path: &Path, sid: &str) -> Result<String, String> {
    let output = Command::new("icacls")
        .arg(path)
        .arg("/grant")
        .arg(format!("*{sid}:(OI)(CI)(F)"))
        .arg("/T")
        .output()
        .map_err(|error| format!("failed to run icacls: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() {
        Ok(format!("icacls ok: {stdout} {stderr}").trim().to_string())
    } else {
        Err(format!(
            "icacls failed ({}): {stdout} {stderr}",
            output.status
        ))
    }
}

/// Try to add the profile to the loopback exemption list. This normally
/// requires elevation; the result is evidence either way.
pub fn add_loopback_exemption(name: &str) -> String {
    let output = Command::new("CheckNetIsolation.exe")
        .args(["LoopbackExempt", "-a", &format!("-n={name}")])
        .output();
    match output {
        Ok(output) => format!(
            "exit={:?} stdout={} stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout).trim(),
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Err(error) => format!("failed to run CheckNetIsolation: {error}"),
    }
}

pub fn remove_loopback_exemption(name: &str) -> String {
    let output = Command::new("CheckNetIsolation.exe")
        .args(["LoopbackExempt", "-d", &format!("-n={name}")])
        .output();
    match output {
        Ok(output) => format!(
            "exit={:?} stdout={} stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout).trim(),
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Err(error) => format!("failed to run CheckNetIsolation: {error}"),
    }
}

pub struct AppContainerChild {
    pub pid: u32,
    process: PROCESS_INFORMATION,
}

impl AppContainerChild {
    pub fn terminate(&mut self) {
        unsafe {
            let _ = windows::Win32::System::Threading::TerminateProcess(self.process.hProcess, 1);
            let _ =
                windows::Win32::System::Threading::WaitForSingleObject(self.process.hProcess, 5000);
            let _ = CloseHandle(self.process.hThread);
            let _ = CloseHandle(self.process.hProcess);
        }
        self.process.hProcess = HANDLE::default();
        self.process.hThread = HANDLE::default();
    }
}

impl Drop for AppContainerChild {
    fn drop(&mut self) {
        if self.process.hProcess != HANDLE::default() {
            self.terminate();
        }
    }
}

/// Launch `exe` with `args` in the named AppContainer, granting the
/// `internetClient` and `privateNetworkClientServer` capability SIDs so the
/// proxy can listen on loopback and reach public hosts.
pub fn launch(
    exe: &Path,
    args: &[String],
    working_directory: &Path,
    profile: &AppContainerProfile,
) -> Result<AppContainerChild, String> {
    let exe_wide = wide(exe.as_os_str());
    let cwd_wide = wide(working_directory.as_os_str());
    let command_line = build_command_line(exe, args);
    let mut command_line_wide: Vec<u16> = command_line.encode_utf16().collect();
    command_line_wide.push(0);

    let mut capability_sids = Vec::new();
    for sid_text in ["S-1-15-3-1", "S-1-15-3-2", "S-1-15-3-3"] {
        let mut sid = PSID::default();
        unsafe {
            ConvertStringSidToSidW(&HSTRING::from(sid_text), &mut sid)
                .map_err(|error| format!("ConvertStringSidToSidW({sid_text}) failed: {error}"))?;
        }
        capability_sids.push(sid);
    }

    let mut capability_entries: Vec<SID_AND_ATTRIBUTES> = capability_sids
        .iter()
        .map(|sid| SID_AND_ATTRIBUTES {
            Sid: *sid,
            Attributes: SE_GROUP_ENABLED as u32,
        })
        .collect();
    let mut security_capabilities = SECURITY_CAPABILITIES {
        AppContainerSid: profile.sid,
        Capabilities: capability_entries.as_mut_ptr(),
        CapabilityCount: capability_entries.len() as u32,
        Reserved: 0,
    };

    let mut attribute_size = 0usize;
    unsafe {
        let _ = InitializeProcThreadAttributeList(None, 1, None, &mut attribute_size);
    }
    if attribute_size == 0 {
        free_capability_sids(&capability_sids);
        return Err("InitializeProcThreadAttributeList did not report a size".to_string());
    }
    let mut attribute_buffer = vec![0u8; attribute_size];
    let list_ptr = attribute_buffer.as_mut_ptr() as *mut core::ffi::c_void;

    let launch_result = (|| -> Result<PROCESS_INFORMATION, String> {
        unsafe {
            InitializeProcThreadAttributeList(
                Some(LPPROC_THREAD_ATTRIBUTE_LIST(list_ptr)),
                1,
                None,
                &mut attribute_size,
            )
            .map_err(|error| format!("InitializeProcThreadAttributeList failed: {error}"))?;

            UpdateProcThreadAttribute(
                LPPROC_THREAD_ATTRIBUTE_LIST(list_ptr),
                0,
                PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
                Some(&mut security_capabilities as *mut _ as *const core::ffi::c_void),
                size_of::<SECURITY_CAPABILITIES>(),
                None,
                None,
            )
            .map_err(|error| format!("UpdateProcThreadAttribute failed: {error}"))?;

            let mut startup = STARTUPINFOEXW::default();
            startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
            startup.lpAttributeList = LPPROC_THREAD_ATTRIBUTE_LIST(list_ptr);

            let mut process = PROCESS_INFORMATION::default();
            CreateProcessW(
                PCWSTR(exe_wide.as_ptr()),
                Some(PWSTR(command_line_wide.as_mut_ptr())),
                None,
                None,
                false,
                EXTENDED_STARTUPINFO_PRESENT,
                None,
                PCWSTR(cwd_wide.as_ptr()),
                &startup.StartupInfo,
                &mut process,
            )
            .map_err(|error| format!("CreateProcessW in AppContainer failed: {error}"))?;
            Ok(process)
        }
    })();

    unsafe {
        DeleteProcThreadAttributeList(LPPROC_THREAD_ATTRIBUTE_LIST(list_ptr));
    }
    free_capability_sids(&capability_sids);

    let process = launch_result?;
    Ok(AppContainerChild {
        pid: process.dwProcessId,
        process,
    })
}

fn free_capability_sids(sids: &[PSID]) {
    for sid in sids {
        unsafe {
            LocalFree(Some(HLOCAL(sid.0)));
        }
    }
}

fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

fn build_command_line(exe: &Path, args: &[String]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(quote_windows_arg(&exe.to_string_lossy()));
    for arg in args {
        parts.push(quote_windows_arg(arg));
    }
    parts.join(" ")
}

fn quote_windows_arg(value: &str) -> String {
    if !value.is_empty() && !value.contains([' ', '\t', '"']) {
        return value.to_string();
    }
    let mut quoted = String::from("\"");
    let mut backslashes = 0usize;
    for character in value.chars() {
        match character {
            '\\' => {
                backslashes += 1;
                quoted.push('\\');
            }
            '"' => {
                quoted.push_str(&"\\".repeat(backslashes));
                quoted.push('\\');
                quoted.push('"');
                backslashes = 0;
            }
            _ => {
                backslashes = 0;
                quoted.push(character);
            }
        }
    }
    quoted.push_str(&"\\".repeat(backslashes));
    quoted.push('"');
    quoted
}

/// Wait for the launched process to exit, up to `timeout`.
pub fn wait_for_exit(child: &AppContainerChild, timeout: std::time::Duration) -> bool {
    unsafe {
        windows::Win32::System::Threading::WaitForSingleObject(
            child.process.hProcess,
            timeout.as_millis() as u32,
        ) == WAIT_OBJECT_0
    }
}
