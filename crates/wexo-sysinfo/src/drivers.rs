//! Best-effort GPU driver version checks.
//!
//! NVIDIA publishes a public lookup endpoint that returns the latest driver
//! for a product series, so its installed version can be compared reliably.
//! Other vendors do not expose a stable machine-readable endpoint; for them the
//! UI still shows the installed driver and a button that opens the official
//! software, while the latest version is reported as unknown instead of being
//! guessed.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use wexo_core::GpuInfo;

/// How long a vendor lookup is reused before asking the server again.
const LOOKUP_TTL: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DriverStatus {
    UpToDate,
    Outdated,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuDriverInfo {
    pub name: String,
    pub vendor: String,
    pub vendor_label: String,
    pub installed_version: String,
    pub latest_version: Option<String>,
    pub status: DriverStatus,
    pub release_date: Option<String>,
    pub details_url: Option<String>,
    pub update_url: String,
    pub software_name: Option<String>,
    pub software_installed: bool,
    pub checked_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Vendor {
    Nvidia,
    Amd,
    Intel,
    Unknown,
}

impl Vendor {
    fn key(self) -> &'static str {
        match self {
            Self::Nvidia => "nvidia",
            Self::Amd => "amd",
            Self::Intel => "intel",
            Self::Unknown => "unknown",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Nvidia => "NVIDIA",
            Self::Amd => "AMD",
            Self::Intel => "Intel",
            Self::Unknown => "GPU",
        }
    }

    fn update_url(self) -> &'static str {
        match self {
            Self::Nvidia => "https://www.nvidia.com/en-us/drivers/",
            Self::Amd => "https://www.amd.com/en/support/download/drivers.html",
            Self::Intel => "https://www.intel.com/content/www/us/en/support/detect.html",
            Self::Unknown => "https://www.nvidia.com/en-us/drivers/",
        }
    }
}

/// Checks every installed GPU. Network work happens only for NVIDIA cards and
/// is cached for a few hours.
pub fn check_all(gpus: &[GpuInfo]) -> Vec<GpuDriverInfo> {
    gpus.iter().map(check_one).collect()
}

/// Checks one GPU. Never panics and never invents a latest version.
pub fn check_one(gpu: &GpuInfo) -> GpuDriverInfo {
    let vendor = vendor_of(gpu);
    let installed = installed_version(gpu, vendor);
    let software = find_software(vendor);

    let mut latest = None;
    let mut release_date = None;
    let mut details_url = None;
    let mut status = DriverStatus::Unknown;

    if vendor == Vendor::Nvidia {
        if let Some(series) = nvidia_series(&gpu.name) {
            if let Some(entry) = nvidia_latest(series) {
                if let Some(older) = version_is_older(&installed, &entry.version) {
                    status = if older {
                        DriverStatus::Outdated
                    } else {
                        DriverStatus::UpToDate
                    };
                }
                latest = Some(entry.version);
                release_date = entry.release_date;
                details_url = entry.details_url;
            }
        }
    }

    GpuDriverInfo {
        name: gpu.name.trim().to_string(),
        vendor: vendor.key().to_string(),
        vendor_label: vendor.label().to_string(),
        installed_version: installed,
        latest_version: latest,
        status,
        release_date,
        details_url,
        update_url: vendor.update_url().to_string(),
        software_name: software.as_ref().map(|(name, _)| name.clone()),
        software_installed: software.is_some(),
        checked_at_ms: now_ms(),
    }
}

/// Opens the manufacturer's update software, falling back to its official page.
/// Returns the software name, or `"browser"` when the page was opened instead.
pub fn open_update(vendor: &str, url: Option<&str>) -> Result<String, String> {
    let vendor = match vendor {
        "nvidia" => Vendor::Nvidia,
        "amd" => Vendor::Amd,
        "intel" => Vendor::Intel,
        _ => Vendor::Unknown,
    };

    if let Some((name, path)) = find_software(vendor) {
        if std::process::Command::new(&path).spawn().is_ok() {
            return Ok(name);
        }
    }

    let target = url
        .filter(|value| is_allowed_url(value))
        .unwrap_or_else(|| vendor.update_url());
    open_url(target)?;
    Ok("browser".to_string())
}

/// Opens a URL in the default browser.
pub fn open_url(url: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let verb: Vec<u16> = "open\0".encode_utf16().collect();
        let target: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                verb.as_ptr(),
                target.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        if result as isize > 32 {
            Ok(())
        } else {
            Err(format!("no se pudo abrir el navegador ({})", result as isize))
        }
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

fn vendor_of(gpu: &GpuInfo) -> Vendor {
    let haystack = format!("{} {}", gpu.vendor, gpu.name).to_ascii_lowercase();
    if haystack.contains("nvidia") {
        Vendor::Nvidia
    } else if haystack.contains("amd")
        || haystack.contains("advanced micro devices")
        || haystack.contains("radeon")
    {
        Vendor::Amd
    } else if haystack.contains("intel") {
        Vendor::Intel
    } else {
        Vendor::Unknown
    }
}

fn installed_version(gpu: &GpuInfo, vendor: Vendor) -> String {
    let raw = gpu.driver_version.trim();
    if raw.is_empty() {
        return String::new();
    }
    if vendor == Vendor::Nvidia {
        nvidia_marketing_version(raw)
    } else {
        raw.to_string()
    }
}

/// NVIDIA reports `32.0.16.1692` in WMI; users know the same driver as
/// `616.92`. The last five digits carry the marketing version.
fn nvidia_marketing_version(raw: &str) -> String {
    let digits: String = raw.chars().filter(char::is_ascii_digit).collect();
    if digits.len() >= 5 {
        let tail = &digits[digits.len() - 5..];
        let (major, minor) = tail.split_at(3);
        if let (Ok(major), Ok(minor)) = (major.parse::<u32>(), minor.parse::<u32>()) {
            return format!("{major}.{minor:02}");
        }
    }
    raw.trim().to_string()
}

/// Maps a GeForce model to NVIDIA's product-series id. Workstation cards use
/// different ids, so they are deliberately left unknown.
fn nvidia_series(name: &str) -> Option<u32> {
    let upper = name.to_ascii_uppercase();
    if upper.contains("QUADRO")
        || upper.contains("TESLA")
        || upper.contains("TITAN")
        || upper.contains("RTX A")
    {
        return None;
    }

    let family = if upper.contains("RTX") {
        "RTX"
    } else if upper.contains("GTX") {
        "GTX"
    } else {
        return None;
    };
    let model = first_model_number(&upper, family)?;
    match (family, model / 100) {
        ("RTX", 50) => Some(131),
        ("RTX", 40) => Some(127),
        ("RTX", 30) => Some(120),
        ("RTX", 20) => Some(107),
        ("GTX", 16) => Some(112),
        ("GTX", 10) => Some(101),
        _ => None,
    }
}

fn first_model_number(text: &str, family: &str) -> Option<u32> {
    let start = text.find(family)? + family.len();
    let digits: String = text[start..]
        .chars()
        .skip_while(|character| !character.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect();
    if digits.len() == 4 {
        digits.parse().ok()
    } else {
        None
    }
}

fn parse_version(value: &str) -> Option<(u32, u32)> {
    let mut parts = value.trim().split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor))
}

fn version_is_older(installed: &str, latest: &str) -> Option<bool> {
    Some(parse_version(installed)? < parse_version(latest)?)
}

#[derive(Debug, Clone)]
struct NvidiaEntry {
    version: String,
    release_date: Option<String>,
    details_url: Option<String>,
}

struct CachedEntry {
    entry: NvidiaEntry,
    fetched_at: Instant,
}

fn nvidia_cache() -> &'static Mutex<std::collections::HashMap<u32, CachedEntry>> {
    static CACHE: OnceLock<Mutex<std::collections::HashMap<u32, CachedEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

fn nvidia_latest(series: u32) -> Option<NvidiaEntry> {
    let cache = nvidia_cache();
    let stale = cache.lock().ok().and_then(|guard| {
        guard.get(&series).and_then(|cached| {
            if cached.fetched_at.elapsed() < LOOKUP_TTL {
                Some(cached.entry.clone())
            } else {
                None
            }
        })
    });
    if stale.is_some() {
        return stale;
    }

    match fetch_nvidia_latest(series) {
        Some(entry) => {
            if let Ok(mut guard) = cache.lock() {
                guard.insert(
                    series,
                    CachedEntry {
                        entry: entry.clone(),
                        fetched_at: Instant::now(),
                    },
                );
            }
            Some(entry)
        }
        // A previous answer is still real data, just older. Prefer it over an
        // empty result when the network is temporarily unavailable.
        None => cache
            .lock()
            .ok()
            .and_then(|guard| guard.get(&series).map(|cached| cached.entry.clone())),
    }
}

fn fetch_nvidia_latest(series: u32) -> Option<NvidiaEntry> {
    let path = format!(
        "/services_toolkit/services/com/nvidia/services/AjaxDriverService.php?func=DriverManualLookup&psid={series}&osID=57&languageCode=1033&isWHQL=1&dch=1&sort1=0&numberOfResults=1"
    );
    let body = http_get("gfwsl.geforce.com", &path)?;
    parse_nvidia_response(&body)
}

fn parse_nvidia_response(body: &str) -> Option<NvidiaEntry> {
    #[derive(serde::Deserialize)]
    struct Response {
        #[serde(rename = "IDS", default)]
        ids: Vec<Id>,
    }

    #[derive(serde::Deserialize)]
    struct Id {
        #[serde(rename = "downloadInfo")]
        download_info: DownloadInfo,
    }

    #[derive(serde::Deserialize)]
    struct DownloadInfo {
        #[serde(rename = "Version", default)]
        version: String,
        #[serde(rename = "ReleaseDateTime", default)]
        release_date: Option<String>,
        #[serde(rename = "DetailsURL", default)]
        details_url: Option<String>,
    }

    let response: Response = serde_json::from_str(body).ok()?;
    let info = response.ids.into_iter().next()?.download_info;
    let version = info.version.trim().to_string();
    parse_version(&version)?;

    Some(NvidiaEntry {
        version,
        release_date: info.release_date.as_deref().and_then(parse_nvidia_date),
        details_url: info.details_url.filter(|url| is_allowed_url(url)),
    })
}

fn parse_nvidia_date(raw: &str) -> Option<String> {
    let mut parts = raw.split_whitespace();
    let _weekday = parts.next()?;
    let month = match parts.next()? {
        "Jan" => "01",
        "Feb" => "02",
        "Mar" => "03",
        "Apr" => "04",
        "May" => "05",
        "Jun" => "06",
        "Jul" => "07",
        "Aug" => "08",
        "Sep" => "09",
        "Oct" => "10",
        "Nov" => "11",
        "Dec" => "12",
        _ => return None,
    };
    let day = parts.next()?.trim_end_matches(',');
    let year = parts.next()?;
    if year.len() != 4 || day.is_empty() || !day.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    Some(format!("{year}-{month}-{day:0>2}"))
}

fn is_allowed_url(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let host = rest
        .split(|character| character == '/' || character == '?' || character == '#')
        .next()
        .unwrap_or_default();
    let host = host.rsplit('@').next().unwrap_or(host);
    let host = host.split(':').next().unwrap_or(host);
    matches!(
        host.to_ascii_lowercase().as_str(),
        "www.nvidia.com"
            | "nvidia.com"
            | "www.amd.com"
            | "amd.com"
            | "www.intel.com"
            | "intel.com"
    )
}

fn find_software(vendor: Vendor) -> Option<(String, PathBuf)> {
    let mut roots = Vec::new();
    for variable in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
        if let Ok(value) = std::env::var(variable) {
            if !value.trim().is_empty() {
                roots.push(PathBuf::from(value));
            }
        }
    }

    let candidates: &[(&str, &str)] = match vendor {
        Vendor::Nvidia => &[
            (
                "NVIDIA App",
                r"NVIDIA Corporation\NVIDIA app\CEF\NVIDIA app.exe",
            ),
            (
                "NVIDIA App",
                r"NVIDIA Corporation\NVIDIA App\CEF\NVIDIA app.exe",
            ),
            (
                "NVIDIA GeForce Experience",
                r"NVIDIA Corporation\NVIDIA GeForce Experience\NVIDIA GeForce Experience.exe",
            ),
        ],
        Vendor::Amd => &[
            (
                "AMD Software: Adrenalin Edition",
                r"AMD\CNext\CNext\RadeonSoftware.exe",
            ),
            (
                "AMD Software: Adrenalin Edition",
                r"AMD\CNext\CNext\amdow.exe",
            ),
        ],
        Vendor::Intel => &[
            ("Intel Arc Control", r"Intel\Intel Arc Control\ArcControl.exe"),
            (
                "Intel Driver & Support Assistant",
                r"Intel\Driver and Support Assistant\DSATray.exe",
            ),
        ],
        Vendor::Unknown => &[],
    };

    for (name, relative) in candidates {
        for root in &roots {
            let path = root.join(relative);
            if path.is_file() {
                return Some(((*name).to_string(), path));
            }
        }
    }
    None
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(windows)]
fn http_get(host: &str, path: &str) -> Option<String> {
    use std::ffi::c_void;
    use std::ptr;
    use windows_sys::Win32::Networking::WinHttp::{
        WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest, WinHttpQueryHeaders,
        WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest, WinHttpSetOption,
        WinHttpSetTimeouts, WINHTTP_ACCESS_TYPE_DEFAULT_PROXY, WINHTTP_FLAG_SECURE,
        WINHTTP_OPTION_REDIRECT_POLICY, WINHTTP_OPTION_REDIRECT_POLICY_DISALLOW_HTTPS_TO_HTTP,
        WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE,
    };

    struct Handle(*mut c_void);

    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                WinHttpCloseHandle(self.0);
            }
        }
    }

    unsafe {
        let agent: Vec<u16> = format!("WexoOptimizer/{}\0", env!("CARGO_PKG_VERSION"))
            .encode_utf16()
            .collect();
        let session = WinHttpOpen(
            agent.as_ptr(),
            WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
            ptr::null(),
            ptr::null(),
            0,
        );
        if session.is_null() {
            return None;
        }
        let session = Handle(session);
        WinHttpSetTimeouts(session.0, 3_000, 3_000, 3_000, 5_000);

        let host_w: Vec<u16> = host.encode_utf16().chain(std::iter::once(0)).collect();
        let connect = WinHttpConnect(session.0, host_w.as_ptr(), 443, 0);
        if connect.is_null() {
            return None;
        }
        let connect = Handle(connect);

        let path_w: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let verb: Vec<u16> = "GET\0".encode_utf16().collect();
        let request = WinHttpOpenRequest(
            connect.0,
            verb.as_ptr(),
            path_w.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            WINHTTP_FLAG_SECURE,
        );
        if request.is_null() {
            return None;
        }
        let request = Handle(request);

        let policy = WINHTTP_OPTION_REDIRECT_POLICY_DISALLOW_HTTPS_TO_HTTP;
        WinHttpSetOption(
            request.0,
            WINHTTP_OPTION_REDIRECT_POLICY,
            &policy as *const u32 as *const c_void,
            std::mem::size_of::<u32>() as u32,
        );

        let headers: Vec<u16> =
            "Accept: application/json, text/html;q=0.9, */*;q=0.8\r\n\0"
                .encode_utf16()
                .collect();
        if WinHttpSendRequest(
            request.0,
            headers.as_ptr(),
            (headers.len() - 1) as u32,
            ptr::null(),
            0,
            0,
            0,
        ) == 0
        {
            return None;
        }
        if WinHttpReceiveResponse(request.0, ptr::null_mut()) == 0 {
            return None;
        }

        let mut status: u32 = 0;
        let mut length = std::mem::size_of::<u32>() as u32;
        if WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            ptr::null(),
            &mut status as *mut u32 as *mut c_void,
            &mut length,
            ptr::null_mut(),
        ) == 0
            || status != 200
        {
            return None;
        }

        let mut body = Vec::new();
        loop {
            let mut buffer = [0u8; 8192];
            let mut read: u32 = 0;
            if WinHttpReadData(
                request.0,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
                &mut read,
            ) == 0
            {
                break;
            }
            if read == 0 {
                break;
            }
            body.extend_from_slice(&buffer[..read as usize]);
            if body.len() > 2 * 1024 * 1024 {
                return None;
            }
        }
        String::from_utf8(body).ok()
    }
}

#[cfg(not(windows))]
fn http_get(_host: &str, _path: &str) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nvidia_versions_are_translated_from_windows_format() {
        assert_eq!(nvidia_marketing_version("32.0.16.1692"), "616.92");
        assert_eq!(nvidia_marketing_version("31.0.15.3699"), "536.99");
        assert_eq!(nvidia_marketing_version("32.0.15.7602"), "576.02");
        assert_eq!(nvidia_marketing_version(""), "");
    }

    #[test]
    fn series_detection_ignores_workstation_cards() {
        assert_eq!(nvidia_series("NVIDIA GeForce RTX 3060"), Some(120));
        assert_eq!(nvidia_series("NVIDIA GeForce RTX 4060 Ti"), Some(127));
        assert_eq!(nvidia_series("NVIDIA GeForce RTX 5090"), Some(131));
        assert_eq!(nvidia_series("NVIDIA GeForce GTX 1660 SUPER"), Some(112));
        assert_eq!(nvidia_series("NVIDIA GeForce GTX 1080 Ti"), Some(101));
        assert_eq!(nvidia_series("NVIDIA Quadro RTX 4000"), None);
        assert_eq!(nvidia_series("NVIDIA RTX A4000"), None);
        assert_eq!(nvidia_series("NVIDIA GeForce GTX 980"), None);
    }

    #[test]
    fn version_comparison_is_conservative() {
        assert_eq!(version_is_older("616.92", "621.14"), Some(true));
        assert_eq!(version_is_older("621.14", "621.14"), Some(false));
        assert_eq!(version_is_older("630.00", "621.14"), Some(false));
        assert_eq!(version_is_older("unknown", "621.14"), None);
    }

    #[test]
    fn parses_the_nvidia_payload() {
        let body = r#"{"Success":"1","IDS":[{"downloadInfo":{"Version":"621.14","ReleaseDateTime":"Wed Nov 04, 2026","DetailsURL":"https://www.nvidia.com/en-us/drivers/details/283969/"}}]}"#;
        let entry = parse_nvidia_response(body).expect("entry");
        assert_eq!(entry.version, "621.14");
        assert_eq!(entry.release_date.as_deref(), Some("2026-11-04"));
        assert_eq!(
            entry.details_url.as_deref(),
            Some("https://www.nvidia.com/en-us/drivers/details/283969/")
        );
    }

    #[test]
    fn rejects_non_official_urls() {
        assert!(is_allowed_url(
            "https://www.nvidia.com/en-us/drivers/details/1/"
        ));
        assert!(is_allowed_url("https://www.amd.com/en/support"));
        assert!(!is_allowed_url("http://www.nvidia.com/"));
        assert!(!is_allowed_url("https://evil.example/?nvidia.com"));
        assert!(!is_allowed_url("https://www.nvidia.com.evil.example/"));
        assert!(!is_allowed_url("javascript:alert(1)"));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "network access"]
    fn nvidia_lookup_works() {
        let entry = fetch_nvidia_latest(120).expect("nvidia response");
        assert!(parse_version(&entry.version).is_some());
    }
}
