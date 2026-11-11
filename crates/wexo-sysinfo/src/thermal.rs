//! CPU and GPU temperatures.
//!
//! Windows has no general temperature API, so each value has its own source and
//! is reported as `None` when that source is not present:
//!
//! * CPU: the ACPI thermal zone performance counters. Many desktops do not
//!   publish a zone; those fall back to LibreHardwareMonitor's web server,
//!   which is started hidden when it is installed and we are elevated.
//! * GPU: NVML, the library installed with the NVIDIA driver.
//!
//! Nothing is estimated here: a missing sensor is not a zero.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// Readings outside this range come from a broken sensor, not from a PC.
const MIN_CELSIUS: f32 = 0.0;
const MAX_CELSIUS: f32 = 125.0;
/// The ACPI counter reports tenths of a degree Kelvin.
const KELVIN_OFFSET: f32 = 273.15;

/// Converts a counter reading (tenths of Kelvin) to Celsius, rejecting values
/// that cannot be a real temperature.
fn celsius_from_tenths_kelvin(value: f64) -> Option<f32> {
    if !value.is_finite() {
        return None;
    }
    let celsius = (value as f32) / 10.0 - KELVIN_OFFSET;
    (MIN_CELSIUS..=MAX_CELSIUS)
        .contains(&celsius)
        .then_some(celsius)
}

/// TCP port of LibreHardwareMonitor's built-in web server.
const LHM_PORT: u16 = 8085;
/// Metric that carries every CPU temperature in the `/metrics` output.
const LHM_CPU_METRIC: &str = "lhm_cpu_temperature_celsius";
/// Sensor names that report the CPU package temperature, most specific first.
/// LibreHardwareMonitor names them differently per vendor.
const LHM_SENSOR_PRIORITY: [&str; 6] = [
    "Core (Tctl/Tdie)",
    "CPU Package",
    "CCDs Max (Tdie)",
    "CCD1 (Tdie)",
    "Core Max",
    "Core Average",
];
/// Hard cap for the `/metrics` body so a misbehaving server cannot exhaust
/// memory while the monitor samples once per second.
const LHM_MAX_BODY: u64 = 512 * 1024;
const LHM_CONNECT_TIMEOUT: Duration = Duration::from_millis(150);
const LHM_READ_TIMEOUT: Duration = Duration::from_millis(400);

/// Reads the CPU temperature from LibreHardwareMonitor's web server.
///
/// The server is local and cheap to query, and it answers with OpenMetrics
/// text using invariant number formatting, so no JSON or locale handling is
/// needed. Any failure is simply "no reading".
fn lhm_celsius() -> Option<f32> {
    let body = fetch_lhm_metrics()?;
    parse_lhm_metrics(&body)
}

fn fetch_lhm_metrics() -> Option<String> {
    let address = SocketAddr::from(([127, 0, 0, 1], LHM_PORT));
    let mut stream = TcpStream::connect_timeout(&address, LHM_CONNECT_TIMEOUT).ok()?;
    stream.set_read_timeout(Some(LHM_READ_TIMEOUT)).ok()?;
    stream.set_write_timeout(Some(LHM_READ_TIMEOUT)).ok()?;

    let request = format!(
        "GET /metrics HTTP/1.1\r\nHost: 127.0.0.1:{LHM_PORT}\r\nAccept: text/plain\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).ok()?;

    let mut response = Vec::new();
    // A kept-alive connection would end with a read timeout after the body;
    // whatever arrived is still parsed, so the error is not fatal.
    let _ = stream.take(LHM_MAX_BODY).read_to_end(&mut response);

    let split = response.windows(4).position(|window| window == b"\r\n\r\n")?;
    let (head, body) = response.split_at(split + 4);
    let head = String::from_utf8_lossy(head);
    if !head.starts_with("HTTP/1.1 200") && !head.starts_with("HTTP/1.0 200") {
        return None;
    }
    Some(String::from_utf8_lossy(body).into_owned())
}

/// Extracts the CPU temperature from the `/metrics` text.
///
/// Each sample is one line, for example:
///
/// ```text
/// lhm_cpu_temperature_celsius {"sensorName"="Core (Tctl/Tdie)", ...} 37.5
/// ```
///
/// The package sensor wins when it is present; otherwise the hottest sensor is
/// used. "Distance to TjMax" sensors are a headroom in degrees, not a
/// temperature, so they are ignored.
fn parse_lhm_metrics(body: &str) -> Option<f32> {
    let mut preferred: Option<(usize, f32)> = None;
    let mut hottest: Option<f32> = None;

    for line in body.lines() {
        let Some(rest) = line.strip_prefix(LHM_CPU_METRIC) else {
            continue;
        };
        let Some(rest) = rest.strip_prefix(' ') else {
            continue;
        };
        let Some(close) = rest.rfind('}') else {
            continue;
        };
        let Ok(value) = rest[close + 1..].trim().parse::<f32>() else {
            continue;
        };
        if !(MIN_CELSIUS..=MAX_CELSIUS).contains(&value) {
            continue;
        }
        let Some(name) = metric_label(&rest[..close], "sensorName") else {
            continue;
        };

        if let Some(rank) = LHM_SENSOR_PRIORITY
            .iter()
            .position(|known| known.eq_ignore_ascii_case(name))
        {
            if preferred.map_or(true, |(best, _)| rank < best) {
                preferred = Some((rank, value));
            }
            continue;
        }
        if name.contains("Distance") {
            continue;
        }
        hottest = Some(hottest.map_or(value, |best: f32| best.max(value)));
    }

    preferred.map(|(_, value)| value).or(hottest)
}

/// Reads one quoted label from a metric's label block.
fn metric_label<'a>(labels: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\"=\"");
    let start = labels.find(&needle)? + needle.len();
    let rest = &labels[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// One reading per device. Either value may be missing.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Temperatures {
    pub cpu_celsius: Option<f32>,
    pub gpu_celsius: Option<f32>,
}

/// Owns the optional temperature sources. Both are opened on the first sample,
/// so a machine without sensors pays a single failed lookup.
#[derive(Default)]
pub struct ThermalSampler {
    zones: Option<imp::ZoneSampler>,
    nvml: Option<imp::Nvml>,
    lhm: imp::Lhm,
    started: bool,
}

impl ThermalSampler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads both sensors, opening them the first time.
    pub fn sample(&mut self) -> Temperatures {
        if !self.started {
            self.started = true;
            self.zones = imp::ZoneSampler::open();
            self.nvml = imp::Nvml::load();
            if self.zones.is_none() {
                tracing::debug!("sin zona térmica ACPI: la CPU se leerá con LibreHardwareMonitor");
            }
            if self.nvml.is_none() {
                tracing::debug!("temperatura de GPU no disponible: NVML no encontrado");
            }
        }

        // The ACPI zone is the cheapest source and already reports the CPU
        // package, so LibreHardwareMonitor is only queried when it is missing.
        let mut cpu_celsius = self.zones.as_mut().and_then(imp::ZoneSampler::celsius);
        if cpu_celsius.is_none() {
            cpu_celsius = self.lhm.celsius();
        }

        Temperatures {
            cpu_celsius,
            gpu_celsius: self.nvml.as_ref().and_then(imp::Nvml::celsius),
        }
    }
}

#[cfg(windows)]
mod imp {
    use std::env;
    use std::ffi::{c_void, OsStr};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::ptr;
    use std::time::Duration;

    use windows_sys::Win32::Foundation::{FreeLibrary, FARPROC, HMODULE, BOOL, HWND, LPARAM};
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows_sys::Win32::System::Performance::{
        PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
        PdhOpenQueryW, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_MORE_DATA,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, ShowWindow, SW_HIDE,
    };

    use super::{celsius_from_tenths_kelvin, MAX_CELSIUS, MIN_CELSIUS};

    const THERMAL_ZONE: &str = r"\Thermal Zone Information(*)\High Precision Temperature";
    const LHM_EXE: &str = "LibreHardwareMonitor.exe";
    /// Package folder WinGet uses for the portable build, relative to
    /// `%LOCALAPPDATA%`.
    const LHM_WINGET_PACKAGE: &str = concat!(
        "Microsoft\\WinGet\\Packages\\",
        "LibreHardwareMonitor.LibreHardwareMonitor_Microsoft.Winget.Source_8wekyb3d8bbwe"
    );

    /// Settings LibreHardwareMonitor needs to answer `/metrics` while staying
    /// out of sight: web server on loopback, no tray icon, start minimized.
    /// Every other preference in its file is left untouched.
    const LHM_SETTINGS: [(&str, &str); 5] = [
        ("runWebServerMenuItem", "true"),
        ("listenerIp", "127.0.0.1"),
        ("listenerPort", "8085"),
        ("startMinMenuItem", "true"),
        ("minTrayMenuItem", "false"),
    ];

    const LHM_DEFAULT_CONFIG: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<configuration>
  <appSettings>
  </appSettings>
</configuration>
"#;

    /// CPU temperature through LibreHardwareMonitor, starting it when it is
    /// installed but not running.
    ///
    /// The program needs administrator rights for its driver, so it is only
    /// started by an elevated process; otherwise an unelevated copy would run
    /// without sensor access and hide the real one.
    pub struct Lhm {
        exe: Option<PathBuf>,
        attempted: bool,
    }

    impl Lhm {
        pub fn new() -> Self {
            Self {
                exe: find_lhm_exe(),
                attempted: false,
            }
        }

        pub fn celsius(&mut self) -> Option<f32> {
            if let Some(celsius) = super::lhm_celsius() {
                return Some(celsius);
            }
            if self.attempted {
                return None;
            }
            self.attempted = true;
            if let Some(exe) = self.exe.as_deref() {
                launch(exe);
            }
            None
        }
    }

    impl Default for Lhm {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Folders the portable build is usually unpacked to, most likely first.
    fn find_lhm_exe() -> Option<PathBuf> {
        let mut candidates = Vec::new();
        if let Some(local) = env::var_os("LOCALAPPDATA") {
            let local = PathBuf::from(local);
            let winget = local.join("Microsoft").join("WinGet");
            candidates.push(
                winget
                    .join("Packages")
                    .join(LHM_WINGET_PACKAGE)
                    .join(LHM_EXE),
            );
            candidates.push(winget.join("Links").join(LHM_EXE));
            candidates.push(
                local
                    .join("Programs")
                    .join("LibreHardwareMonitor")
                    .join(LHM_EXE),
            );
        }
        for variable in ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"] {
            if let Some(root) = env::var_os(variable) {
                candidates.push(
                    PathBuf::from(root)
                        .join("LibreHardwareMonitor")
                        .join(LHM_EXE),
                );
            }
        }
        if let Some(found) = candidates.into_iter().find(|path| path.is_file()) {
            return Some(found);
        }
        // WinGet also drops a shim in a folder on PATH.
        let path = env::var_os("PATH")?;
        env::split_paths(&path)
            .map(|folder| folder.join(LHM_EXE))
            .find(|candidate| candidate.is_file())
    }

    /// Starts LibreHardwareMonitor hidden, ready to be queried a few seconds
    /// later. Returns whether a new process was created.
    fn launch(exe: &Path) -> bool {
        if !crate::is_elevated() {
            tracing::debug!(
                "temperatura de CPU: LibreHardwareMonitor necesita ejecutarse como administrador"
            );
            return false;
        }
        if process_running() {
            tracing::debug!(
                "temperatura de CPU: LibreHardwareMonitor está abierto sin el servidor web"
            );
            return false;
        }
        // The WinGet shim is a link; the settings live next to the real exe.
        let exe = fs::canonicalize(exe).unwrap_or_else(|_| exe.to_path_buf());
        if !ensure_config(&exe) {
            return false;
        }
        let spawned = Command::new(&exe)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        match spawned {
            Ok(child) => {
                // `minTrayMenuItem=false` keeps the tray icon away, but the
                // program then starts minimized in the taskbar. Hide that
                // window too so only the Wexo overlay is visible.
                hide_window_when_ready(child.id());
                tracing::info!("LibreHardwareMonitor iniciado en segundo plano para leer la CPU");
                true
            }
            Err(error) => {
                tracing::warn!(%error, "no se pudo iniciar LibreHardwareMonitor");
                false
            }
        }
    }

    /// Hides LibreHardwareMonitor's top-level windows as soon as they exist.
    ///
    /// The program has no headless mode, so the window is created and then
    /// hidden while its web server keeps sampling. A short retry loop covers
    /// the time WinForms needs to create the form.
    fn hide_window_when_ready(pid: u32) {
        let spawned = std::thread::Builder::new()
            .name("wexo-lhm-hide".to_string())
            .spawn(move || {
                for _ in 0..50 {
                    if hide_process_windows(pid) {
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                tracing::debug!("no se encontró la ventana de LibreHardwareMonitor para ocultarla");
            });
        if let Err(error) = spawned {
            tracing::debug!(%error, "no se pudo lanzar el hilo que oculta LibreHardwareMonitor");
        }
    }

    /// Hides every top-level window owned by `pid`. Returns whether any was
    /// found, so the caller knows when to stop retrying.
    fn hide_process_windows(pid: u32) -> bool {
        struct Search {
            pid: u32,
            found: bool,
        }

        unsafe extern "system" fn callback(window: HWND, lparam: LPARAM) -> BOOL {
            // SAFETY: `lparam` is the `&mut Search` passed to `EnumWindows`,
            // which only lives for the duration of the call.
            let search = unsafe { &mut *(lparam as *mut Search) };
            let mut owner = 0u32;
            // SAFETY: `window` is a valid handle supplied by the system.
            unsafe { GetWindowThreadProcessId(window, &mut owner) };
            if owner == search.pid {
                // SAFETY: the handle belongs to the target process and hiding
                // it does not invalidate anything we hold.
                unsafe { ShowWindow(window, SW_HIDE) };
                search.found = true;
            }
            1
        }

        let mut search = Search { pid, found: false };
        // SAFETY: the callback and the pointer stay valid until it returns.
        unsafe { EnumWindows(Some(callback), &mut search as *mut Search as LPARAM) };
        search.found
    }

    fn process_running() -> bool {
        let mut system = sysinfo::System::new();
        system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        let running = system
            .processes_by_name(OsStr::new("LibreHardwareMonitor"))
            .next()
            .is_some();
        running
    }

    /// Settings file LibreHardwareMonitor reads: its executable path with a
    /// `.config` extension.
    pub fn config_path(exe: &Path) -> PathBuf {
        exe.with_extension("config")
    }

    /// Makes sure the web server and the minimized start are enabled.
    ///
    /// Returns `false` when the file cannot be written or does not look like
    /// the program's own XML, so an unrelated file is never clobbered.
    fn ensure_config(exe: &Path) -> bool {
        let path = config_path(exe);
        let existing = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                LHM_DEFAULT_CONFIG.to_string()
            }
            Err(_) => return false,
        };
        let Some(merged) = merge_settings(&existing) else {
            return false;
        };
        merged == existing || fs::write(&path, merged).is_ok()
    }

    /// Adds or updates the required settings, preserving the rest of the file.
    pub fn merge_settings(config: &str) -> Option<String> {
        let marker = "<appSettings>";
        config.find(marker)?;
        let mut merged = config.to_string();
        let mut missing = String::new();

        for (key, value) in LHM_SETTINGS {
            if merged.contains(&format!("key=\"{key}\"")) {
                if let Some(updated) = replace_value(&merged, key, value) {
                    merged = updated;
                }
            } else {
                missing.push_str(&format!("\n    <add key=\"{key}\" value=\"{value}\" />"));
            }
        }

        if !missing.is_empty() {
            let insert_at = merged.find(marker)? + marker.len();
            merged.insert_str(insert_at, &missing);
        }
        Some(merged)
    }

    /// Replaces the `value` attribute of the `<add key="…" />` element.
    fn replace_value(config: &str, key: &str, value: &str) -> Option<String> {
        let key_at = config.find(&format!("key=\"{key}\""))?;
        let element = &config[key_at..];
        let element = &element[..element.find("/>")?];
        let value_at = element.find("value=\"")? + "value=\"".len();
        let value_len = element[value_at..].find('"')?;
        let start = key_at + value_at;
        let end = start + value_len;
        Some(format!("{}{}{}", &config[..start], value, &config[end..]))
    }

    /// Hottest ACPI thermal zone, which is the CPU zone on machines that
    /// publish one.
    pub struct ZoneSampler {
        query: isize,
        counter: isize,
        /// `u64` so the PDH item array stays 8-byte aligned.
        buffer: Vec<u64>,
    }

    impl ZoneSampler {
        pub fn open() -> Option<Self> {
            let mut sampler = Self {
                query: 0,
                counter: 0,
                buffer: Vec::new(),
            };
            unsafe {
                if PdhOpenQueryW(ptr::null(), 0, &mut sampler.query) != 0 {
                    return None;
                }
                let path: Vec<u16> = THERMAL_ZONE
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                if PdhAddEnglishCounterW(sampler.query, path.as_ptr(), 0, &mut sampler.counter) != 0
                {
                    PdhCloseQuery(sampler.query);
                    return None;
                }
                // Array counters need one collection before the first read.
                PdhCollectQueryData(sampler.query);
            }
            Some(sampler)
        }

        pub fn celsius(&mut self) -> Option<f32> {
            unsafe {
                if PdhCollectQueryData(self.query) != 0 {
                    return None;
                }

                let mut size: u32 = 0;
                let mut count: u32 = 0;
                let status = PdhGetFormattedCounterArrayW(
                    self.counter,
                    PDH_FMT_DOUBLE,
                    &mut size,
                    &mut count,
                    ptr::null_mut(),
                );
                if status != PDH_MORE_DATA || size == 0 {
                    return None;
                }

                let words = (size as usize).div_ceil(8);
                self.buffer.clear();
                self.buffer.resize(words, 0);

                let items = self.buffer.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
                if PdhGetFormattedCounterArrayW(
                    self.counter,
                    PDH_FMT_DOUBLE,
                    &mut size,
                    &mut count,
                    items,
                ) != 0
                {
                    return None;
                }

                let entries = std::slice::from_raw_parts(items, count as usize);
                entries
                    .iter()
                    // PDH_CSTATUS_VALID_DATA (0) or PDH_CSTATUS_NEW_DATA (1).
                    .filter(|entry| entry.FmtValue.CStatus <= 1)
                    .filter_map(|entry| {
                        celsius_from_tenths_kelvin(entry.FmtValue.Anonymous.doubleValue)
                    })
                    .reduce(f32::max)
            }
        }
    }

    impl Drop for ZoneSampler {
        fn drop(&mut self) {
            unsafe {
                if self.query != 0 {
                    PdhCloseQuery(self.query);
                }
            }
        }
    }

    type NvmlReturn = i32;
    const NVML_SUCCESS: NvmlReturn = 0;
    /// `NVML_TEMPERATURE_GPU` from `nvml.h`.
    const NVML_TEMPERATURE_GPU: i32 = 0;

    type NvmlInit = unsafe extern "system" fn() -> NvmlReturn;
    type NvmlShutdown = unsafe extern "system" fn() -> NvmlReturn;
    type NvmlCount = unsafe extern "system" fn(*mut u32) -> NvmlReturn;
    type NvmlHandle = unsafe extern "system" fn(u32, *mut *mut c_void) -> NvmlReturn;
    type NvmlTemperature = unsafe extern "system" fn(*mut c_void, i32, *mut u32) -> NvmlReturn;

    /// The NVML library that ships with the NVIDIA driver, loaded by hand so
    /// machines without it keep working.
    pub struct Nvml {
        module: HMODULE,
        shutdown: NvmlShutdown,
        count: NvmlCount,
        handle: NvmlHandle,
        temperature: NvmlTemperature,
    }

    impl Nvml {
        pub fn load() -> Option<Self> {
            unsafe {
                let module = LoadLibraryW(wide("nvml.dll").as_ptr());
                if module.is_null() {
                    return None;
                }

                let symbols = (|| {
                    Some((
                        symbol::<NvmlInit>(module, b"nvmlInit_v2\0")?,
                        symbol::<NvmlShutdown>(module, b"nvmlShutdown\0")?,
                        symbol::<NvmlCount>(module, b"nvmlDeviceGetCount_v2\0")?,
                        symbol::<NvmlHandle>(module, b"nvmlDeviceGetHandleByIndex_v2\0")?,
                        symbol::<NvmlTemperature>(module, b"nvmlDeviceGetTemperature\0")?,
                    ))
                })();
                let Some((init, shutdown, count, handle, temperature)) = symbols else {
                    FreeLibrary(module);
                    return None;
                };
                if init() != NVML_SUCCESS {
                    FreeLibrary(module);
                    return None;
                }

                Some(Self {
                    module,
                    shutdown,
                    count,
                    handle,
                    temperature,
                })
            }
        }

        /// Hottest GPU in Celsius, or `None` when the driver cannot report it.
        pub fn celsius(&self) -> Option<f32> {
            unsafe {
                let mut count = 0u32;
                if (self.count)(&mut count) != NVML_SUCCESS || count == 0 {
                    return None;
                }

                let mut hottest: Option<f32> = None;
                for index in 0..count {
                    let mut device: *mut c_void = ptr::null_mut();
                    if (self.handle)(index, &mut device) != NVML_SUCCESS || device.is_null() {
                        continue;
                    }
                    let mut value = 0u32;
                    if (self.temperature)(device, NVML_TEMPERATURE_GPU, &mut value) != NVML_SUCCESS
                    {
                        continue;
                    }
                    let celsius = value as f32;
                    if !(MIN_CELSIUS..=MAX_CELSIUS).contains(&celsius) {
                        continue;
                    }
                    hottest = Some(hottest.map_or(celsius, |best| best.max(celsius)));
                }
                hottest
            }
        }
    }

    impl Drop for Nvml {
        fn drop(&mut self) {
            unsafe {
                (self.shutdown)();
                FreeLibrary(self.module);
            }
        }
    }

    // SAFETY: the module handle and the NVML entry points are process-wide and
    // NVML is documented as thread-safe; the sampler lives on the monitor
    // thread and never shares the handle.
    unsafe impl Send for Nvml {}

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    unsafe fn symbol<T: Copy>(module: HMODULE, name: &[u8]) -> Option<T> {
        let address: FARPROC = GetProcAddress(module, name.as_ptr());
        let address = address?;
        Some(std::mem::transmute_copy(&address))
    }
}

#[cfg(not(windows))]
mod imp {
    /// On other platforms LibreHardwareMonitor does not exist, but the shared
    /// probe keeps working if something answers on its port.
    #[derive(Default)]
    pub struct Lhm;

    impl Lhm {
        pub fn new() -> Self {
            Self
        }

        pub fn celsius(&mut self) -> Option<f32> {
            super::lhm_celsius()
        }
    }

    pub struct ZoneSampler;

    impl ZoneSampler {
        pub fn open() -> Option<Self> {
            None
        }

        pub fn celsius(&mut self) -> Option<f32> {
            None
        }
    }

    pub struct Nvml;

    impl Nvml {
        pub fn load() -> Option<Self> {
            None
        }

        pub fn celsius(&self) -> Option<f32> {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kelvin_readings_become_celsius() {
        // 300.0 K is 26.85 °C.
        let celsius = celsius_from_tenths_kelvin(3000.0).expect("celsius");
        assert!((celsius - 26.85).abs() < 0.01, "celsius = {celsius}");
    }

    #[test]
    fn implausible_readings_are_rejected() {
        assert_eq!(celsius_from_tenths_kelvin(0.0), None);
        assert_eq!(celsius_from_tenths_kelvin(5000.0), None);
        assert_eq!(celsius_from_tenths_kelvin(f64::NAN), None);
        assert_eq!(celsius_from_tenths_kelvin(f64::INFINITY), None);
    }

    #[test]
    fn sampling_without_sensors_reports_nothing() {
        let mut sampler = ThermalSampler::new();
        let temperatures = sampler.sample();
        for value in [temperatures.cpu_celsius, temperatures.gpu_celsius]
            .into_iter()
            .flatten()
        {
            assert!(
                (MIN_CELSIUS..=MAX_CELSIUS).contains(&value),
                "value = {value}"
            );
        }
    }

    const AMD_METRICS: &str = r#"# TYPE lhm_cpu_temperature_celsius gauge
lhm_cpu_temperature_celsius {"sensorName"="Core (Tctl/Tdie)", "hardwareId"="/amdcpu/0"} 37.5
lhm_cpu_temperature_celsius {"sensorName"="SoC", "hardwareId"="/amdcpu/0"} 34.5
lhm_cpu_temperature_celsius {"sensorName"="CCD1 (Tdie)", "hardwareId"="/amdcpu/0"} 39
lhm_cpu_temperature_celsius {"sensorName"="CCDs Max (Tdie)", "hardwareId"="/amdcpu/0"} 40.5
"#;

    #[test]
    fn lhm_prefers_the_package_sensor() {
        assert_eq!(parse_lhm_metrics(AMD_METRICS), Some(37.5));
    }

    #[test]
    fn lhm_prefers_the_intel_package_over_single_cores() {
        let body = r#"lhm_cpu_temperature_celsius {"sensorName"="Core #1"} 62
lhm_cpu_temperature_celsius {"sensorName"="Core Max"} 61
lhm_cpu_temperature_celsius {"sensorName"="CPU Package"} 55
"#;
        assert_eq!(parse_lhm_metrics(body), Some(55.0));
    }

    #[test]
    fn lhm_falls_back_to_the_hottest_known_sensor() {
        let body = r#"lhm_cpu_temperature_celsius {"sensorName"="Core #1"} 41
lhm_cpu_temperature_celsius {"sensorName"="Core #2"} 44.5
lhm_cpu_temperature_celsius {"sensorName"="Distance to TjMax"} 20
"#;
        assert_eq!(parse_lhm_metrics(body), Some(44.5));
    }

    #[test]
    fn lhm_ignores_implausible_readings() {
        let body = r#"lhm_cpu_temperature_celsius {"sensorName"="Core (Tctl/Tdie)"} 200
lhm_cpu_temperature_celsius {"sensorName"="CCD1 (Tdie)"} -5
"#;
        assert_eq!(parse_lhm_metrics(body), None);
    }

    #[test]
    fn lhm_reports_nothing_without_cpu_sensors() {
        let body = r#"lhm_gpunvidia_temperature_celsius {"sensorName"="GPU Core"} 50
lhm_cpu_temperature_celsius {"sensorName"="Core"} not-a-number
"#;
        assert_eq!(parse_lhm_metrics(body), None);
    }

    #[cfg(windows)]
    mod config {
        use super::imp;
        use std::path::Path;

        const EXISTING: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<configuration>
  <appSettings>
    <add key="mainboardMenuItem" value="false" />
    <add key="runWebServerMenuItem" value="false" />
  </appSettings>
</configuration>
"#;

        #[test]
        fn settings_file_sits_next_to_the_executable() {
            let path = imp::config_path(Path::new(r"C:\Tools\LibreHardwareMonitor.exe"));
            assert!(path.ends_with("LibreHardwareMonitor.config"), "{path:?}");
        }

        #[test]
        fn web_server_settings_are_added_and_kept() {
            let merged = imp::merge_settings(EXISTING).expect("merged");
            for key in [
                "runWebServerMenuItem\" value=\"true",
                "listenerIp\" value=\"127.0.0.1",
                "listenerPort\" value=\"8085",
                "startMinMenuItem\" value=\"true",
                "minTrayMenuItem\" value=\"false",
            ] {
                assert!(merged.contains(key), "missing {key} in {merged}");
            }
            assert!(merged.contains(r#"<add key="mainboardMenuItem" value="false" />"#));
            assert!(!merged.contains(r#"key="runWebServerMenuItem" value="false""#));
            assert_eq!(imp::merge_settings(&merged).as_deref(), Some(merged.as_str()));
        }

        #[test]
        fn unknown_files_are_left_alone() {
            assert_eq!(imp::merge_settings("<configuration />"), None);
        }
    }
}
