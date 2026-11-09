#![windows_subsystem = "windows"]

use std::env;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;

fn find_python() -> Option<PathBuf> {
    // 1. Check known Python directories in LOCALAPPDATA (installed via python.org installer)
    if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
        let programs = PathBuf::from(&local_app_data).join("Programs").join("Python");
        if programs.exists() {
            if let Ok(entries) = std::fs::read_dir(programs) {
                for entry in entries.flatten() {
                    let pw = entry.path().join("pythonw.exe");
                    if pw.exists() && pw.metadata().map(|m| m.len() > 0).unwrap_or(false) {
                        return Some(pw);
                    }
                    let p = entry.path().join("python.exe");
                    if p.exists() && p.metadata().map(|m| m.len() > 0).unwrap_or(false) {
                        return Some(p);
                    }
                }
            }
        }
    }

    // 2. Query where.exe for pythonw.exe (prefer non-WindowsApps)
    if let Ok(output) = Command::new("where.exe").arg("pythonw.exe").output() {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            for line in s.lines() {
                let p = PathBuf::from(line.trim());
                if p.exists() && !line.contains("WindowsApps") {
                    return Some(p);
                }
            }
            if let Some(line) = s.lines().next() {
                let p = PathBuf::from(line.trim());
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }

    // 3. Query where.exe for python.exe
    if let Ok(output) = Command::new("where.exe").arg("python.exe").output() {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            for line in s.lines() {
                let p = PathBuf::from(line.trim());
                if p.exists() && !line.contains("WindowsApps") {
                    let pw = p.with_file_name("pythonw.exe");
                    if pw.exists() {
                        return Some(pw);
                    }
                    return Some(p);
                }
            }
            if let Some(line) = s.lines().next() {
                let p = PathBuf::from(line.trim());
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }

    // 4. Check common C:\Program Files or C:\Python*
    for root in &[
        "C:\\Python312",
        "C:\\Python311",
        "C:\\Python310",
        "C:\\Program Files\\Python312",
        "C:\\Program Files\\Python311",
        "C:\\Program Files\\Python310",
    ] {
        let pw = Path::new(root).join("pythonw.exe");
        if pw.exists() {
            return Some(pw);
        }
        let p = Path::new(root).join("python.exe");
        if p.exists() {
            return Some(p);
        }
    }

    None
}

fn show_error(title: &str, msg: &str) {
    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(
            hwnd: *mut std::ffi::c_void,
            lp_text: *const u16,
            lp_caption: *const u16,
            u_type: u32,
        ) -> i32;
    }
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    let text_wide: Vec<u16> = OsStr::new(msg).encode_wide().chain(Some(0)).collect();
    let title_wide: Vec<u16> = OsStr::new(title).encode_wide().chain(Some(0)).collect();

    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text_wide.as_ptr(),
            title_wide.as_ptr(),
            0x10, // MB_ICONERROR
        );
    }
}

fn main() {
    let current_exe = match env::current_exe() {
        Ok(p) => p,
        Err(_) => PathBuf::from("."),
    };
    let app_dir = current_exe.parent().unwrap_or(Path::new("."));

    // Find desktop/app.py relative to the exe or current dir
    let app_py_candidates = [
        app_dir.join("desktop").join("app.py"),
        app_dir.join("app.py"),
        PathBuf::from("desktop/app.py"),
    ];

    let mut target_script = None;
    for cand in &app_py_candidates {
        if cand.exists() {
            target_script = Some(cand.clone());
            break;
        }
    }

    let target_script = match target_script {
        Some(s) => s,
        None => {
            show_error(
                "SonixLink Error",
                &format!(
                    "Cannot locate 'desktop/app.py'.\nPlease ensure SonixLink.exe is located inside the application directory.\nLooked in: {:?}",
                    app_dir
                ),
            );
            return;
        }
    };

    let python = match find_python() {
        Some(p) => p,
        None => {
            show_error(
                "Python Not Found",
                "SonixLink Desktop requires Python 3.10+.\nPlease install Python from python.org or the Microsoft Store and ensure it is added to your PATH.",
            );
            return;
        }
    };

    let mut cmd = Command::new(&python);
    cmd.arg(&target_script);

    // Pass through any CLI arguments
    for arg in env::args().skip(1) {
        cmd.arg(arg);
    }

    cmd.current_dir(target_script.parent().unwrap_or(app_dir));
    cmd.creation_flags(CREATE_NO_WINDOW);

    if let Err(e) = cmd.spawn() {
        show_error(
            "Launch Failed",
            &format!("Failed to launch SonixLink Desktop:\n{}", e),
        );
    }
}
