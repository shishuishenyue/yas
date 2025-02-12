use std::ffi::{OsStr};
use std::os::windows::ffi::OsStrExt;
use std::iter::once;
use std::ptr::null_mut;
use std::{thread, time};
use std::fs;
use std::io::stdin;
use std::process;
use std::time::Duration;

use log::{error, info};
use reqwest::blocking::Client;
use reqwest::ClientBuilder;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use winapi::um::winuser::{FindWindowW, GetClientRect, ClientToScreen, GetAsyncKeyState, VK_RBUTTON};
use winapi::shared::windef::{HWND, RECT as WinRect, POINT as WinPoint};

use crate::common::PixelRect;
use winapi::um::winnt::{SID_IDENTIFIER_AUTHORITY, SECURITY_NT_AUTHORITY, PSID, SECURITY_BUILTIN_DOMAIN_RID, DOMAIN_ALIAS_RID_ADMINS};
use winapi::um::securitybaseapi::{AllocateAndInitializeSid, CheckTokenMembership, FreeSid};
use winapi::shared::minwindef::BOOL;
use crate::dto::GithubTag;

pub fn encode_wide(s: String) -> Vec<u16> {
    let wide: Vec<u16> = OsStr::new(&s).encode_wide().chain(once(0)).collect();
    wide
}

pub fn find_window(title: String) -> Result<HWND, String> {
    let wide = encode_wide(title);
    let result: HWND = unsafe {
        FindWindowW(null_mut(), wide.as_ptr())
    };
    if result.is_null() {
        Err(String::from("cannot find window"))
    } else {
        Ok(result)
    }
}

unsafe fn get_client_rect_unsafe(hwnd: HWND) -> Result<PixelRect, String> {
    let mut rect: WinRect = WinRect {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    GetClientRect(hwnd, &mut rect);
    let width: i32 = rect.right;
    let height: i32 = rect.bottom;

    let mut point: WinPoint = WinPoint {
        x: 0,
        y: 0,
    };
    ClientToScreen(hwnd, &mut point as *mut WinPoint);
    let left: i32 = point.x;
    let top: i32 = point.y;

    Ok(PixelRect {
        left, top,
        width, height
    })
}

pub fn get_client_rect(hwnd: HWND) -> Result<PixelRect, String> {
    unsafe {
        get_client_rect_unsafe(hwnd)
    }
}

pub fn sleep(ms: u32) {
    let time = time::Duration::from_millis(ms as u64);
    thread::sleep(time);
}

pub fn read_file_to_string(path: String) -> String {
    let content = fs::read_to_string(path).unwrap();
    content
}

pub fn error_and_quit(msg: &str) -> ! {
    error!("{}, 按Enter退出", msg);
    let mut s: String = String::new();
    stdin().read_line(&mut s);
    process::exit(0);
}

unsafe fn is_admin_unsafe() -> bool {
    let mut authority: SID_IDENTIFIER_AUTHORITY = SID_IDENTIFIER_AUTHORITY {
        Value: SECURITY_NT_AUTHORITY,
    };
    let mut group: PSID = null_mut();
    let mut b = AllocateAndInitializeSid(
        &mut authority as *mut SID_IDENTIFIER_AUTHORITY,
        2,
        SECURITY_BUILTIN_DOMAIN_RID,
        DOMAIN_ALIAS_RID_ADMINS,
        0, 0, 0, 0, 0, 0,
        &mut group as *mut PSID,
    );
    if b != 0 {
        let r = CheckTokenMembership(null_mut(), group, &mut b as *mut BOOL);
        if r == 0 {
            b = 0;
        }
        FreeSid(group);
    }

    b != 0
}

pub fn is_admin() -> bool {
    unsafe {
        is_admin_unsafe()
    }
}

pub fn is_rmb_down() -> bool {
    unsafe {
        let state = GetAsyncKeyState(VK_RBUTTON);
        if state == 0 {
            return false;
        }

        state & 1 > 0
    }
}

static mut VERSION: String = String::new();
unsafe fn get_version_unsafe() -> String {
    if VERSION.len() == 0 {
        let s = include_str!("../../Cargo.toml");
        for line in s.lines() {
            if line.starts_with("version = ") {
                let temp = line.split("\"").collect::<Vec<_>>();
                let version = String::from(temp[temp.len() - 2]);
                VERSION = version;
            }
        }
    }

    VERSION.clone()
}

pub fn get_version() -> String {
    unsafe {
        get_version_unsafe()
    }
}

pub fn check_update() -> Option<String> {
    let client = Client::new();

    let resp = client.get("https://api.github.com/repos/wormtql/yas/tags")
        .timeout(Duration::from_secs(5))
        .header(USER_AGENT, HeaderValue::from_static("reqwest"))
        .send().ok()?.json::<Vec<GithubTag>>().ok()?;

    let latest = if resp.len() == 0 {
        return None
    } else {
        resp[0].name.clone()
    };
    let latest = &latest[1..];

    let latest_sem: semver::Version = semver::Version::parse(&latest).unwrap();
    let current_sem: semver::Version = semver::Version::parse(&get_version()).unwrap();

    if latest_sem > current_sem {
        Some(String::from(latest))
    } else {
        None
    }
}
