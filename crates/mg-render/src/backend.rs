//! Which graphics adapter, and so which backend (Vulkan, Direct3D 12,
//! OpenGL, Metal), Moonglow draws with: the window's (`apps/moonglow`,
//! by Options › General › Graphics backend) and the headless one's
//! ([`Gpu::headless`](crate::Gpu::headless)), chosen here among the
//! adapters wgpu found, by what each says of itself, so that the choice
//! is tested without a GPU.
//!
//! On Automatic it is wgpu's own choice (its `request_adapter`: the
//! adapters in the order the backends list them, sorted by kind for the
//! power preference) with one exception: on Windows, an AMD GPU is taken
//! through Direct3D 12 rather than Vulkan. AMD's Windows Vulkan driver
//! drew areas wrong (their tiles shown when an area opened, then gone one
//! by one; Radeon PRO, driver 25.10.37.03, October 2026), and the same GPU
//! through Direct3D 12 drew them right. But another builder's Radeon RX 580
//! drew nothing of an area through Direct3D 12 (since 2.11.0), and through
//! Vulkan drew it right: no fixed rule serves both. So the adapter taken is
//! checked as Moonglow starts ([`start`], with [`crate::probe`]), and where
//! it draws nothing on Automatic the same GPU's other backend is taken; a
//! backend that failed the check, or the area view's watch
//! ([`crate::watch`]), is kept in the settings ([`Failed`]) and passed over
//! by Automatic until the driver changes.

use std::fmt;

use wgpu::{AdapterInfo, Backend, DeviceType, PowerPreference};

/// AMD's PCI vendor ID (4098).
pub const AMD: u32 = 0x1002;

/// The operating systems whose backends differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Linux,
    MacOs,
    /// Another Unix (the BSDs): as Linux.
    Other,
}

impl Platform {
    /// The one Moonglow runs on.
    pub fn current() -> Platform {
        if cfg!(windows) {
            Platform::Windows
        } else if cfg!(target_os = "macos") {
            Platform::MacOs
        } else if cfg!(target_os = "linux") {
            Platform::Linux
        } else {
            Platform::Other
        }
    }

    /// The backends that can be chosen here, in wgpu's order. (Not
    /// OpenGL: Moonglow's renderer draws particles, the game's shaders and
    /// more wrong on wgpu's GL backend. wgpu still falls back to it where
    /// nothing else draws, as it did before the choice.)
    pub fn backends(self) -> &'static [Backend] {
        match self {
            Platform::Windows => &[Backend::Vulkan, Backend::Dx12],
            Platform::MacOs => &[Backend::Metal],
            Platform::Linux | Platform::Other => &[Backend::Vulkan],
        }
    }
}

/// A backend's name as the builder reads it.
pub fn backend_name(backend: Backend) -> &'static str {
    match backend {
        Backend::Vulkan => "Vulkan",
        Backend::Dx12 => "Direct3D 12",
        Backend::Gl => "OpenGL",
        Backend::Metal => "Metal",
        Backend::BrowserWebGpu => "WebGPU",
        Backend::Noop => "no backend",
    }
}

/// Why an adapter was taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// The backend chosen in Options.
    Chosen(Backend),
    /// `WGPU_BACKEND` named the backends: wgpu's order among them.
    Environment,
    /// Automatic: wgpu's order.
    Automatic,
    /// Automatic: an AMD GPU on Windows, taken through Direct3D 12 rather
    /// than Vulkan.
    AmdOnWindows,
}

/// The adapter taken: its index among those given, why, the backend
/// chosen in Options that had no adapter (Automatic was used instead), and
/// the backends Automatic passed over as recorded failures ([`Failed`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pick {
    pub index: usize,
    pub why: Why,
    pub missing: Option<Backend>,
    pub passed_over: Vec<Backend>,
    /// The adapter is none Moonglow draws right or fast with
    /// ([`Fallback`]): taken as nothing else was there, or as asked for.
    pub fallback: Option<Fallback>,
}

/// An adapter Automatic takes only where there is no other: the views are
/// drawn wrong or slowly with it, and the builder is told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fallback {
    /// A backend the platform's choices leave out (OpenGL: Moonglow's
    /// renderer draws particles, the game's shaders and more wrong on
    /// wgpu's).
    Backend(Backend),
    /// A software adapter (llvmpipe, Microsoft's Basic Render Driver): a
    /// few frames a second.
    Software,
}

impl Fallback {
    /// What `adapter` is a fallback as, on `platform` (`None`: an adapter
    /// Automatic draws with).
    pub fn of(adapter: &AdapterInfo, platform: Platform) -> Option<Fallback> {
        if !platform.backends().contains(&adapter.backend) {
            Some(Fallback::Backend(adapter.backend))
        } else if adapter.device_type == DeviceType::Cpu {
            Some(Fallback::Software)
        } else {
            None
        }
    }

    /// What it is, and what to do about it, for the log and the notice
    /// (after "Moonglow draws through Vulkan on llvmpipe, ").
    pub fn said(self, platform: Platform) -> String {
        let what = match self {
            Fallback::Backend(b) => format!(
                "and its renderer draws parts of the 3D views wrong through {}",
                backend_name(b)
            ),
            Fallback::Software => {
                "a software adapter, which draws the 3D views at a few frames a second".into()
            }
        };
        let wanted: Vec<&str> = platform.backends().iter().map(|b| backend_name(*b)).collect();
        let install = match platform {
            Platform::Windows => {
                "Install the graphics driver of the GPU's maker (over a remote desktop or in a                  virtual machine there may be none)."
            }
            Platform::MacOs => "This Mac's GPU has no Metal driver.",
            Platform::Linux | Platform::Other => {
                "Install the GPU's Vulkan driver (the distribution's Mesa Vulkan drivers package,                  or the GPU maker's driver)."
            }
        };
        format!("{what}: no GPU that draws through {} was found. {install}", wanted.join(" or "))
    }
}

impl Pick {
    /// Whether the backend was asked for (Options, `WGPU_BACKEND`): kept
    /// though it fails the check.
    pub fn asked_for(&self) -> bool {
        matches!(self.why, Why::Chosen(_) | Why::Environment)
    }
}

impl fmt::Display for Pick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(missing) = self.missing {
            write!(f, "no {} adapter for the backend chosen, so ", backend_name(missing))?;
        }
        match self.why {
            Why::Chosen(b) => write!(f, "{} chosen in Options", backend_name(b))?,
            Why::Environment => write!(f, "WGPU_BACKEND")?,
            Why::Automatic => write!(f, "Automatic (wgpu's order)")?,
            Why::AmdOnWindows => {
                write!(f, "Automatic (Direct3D 12 rather than Vulkan for AMD on Windows)")?
            }
        }
        if !self.passed_over.is_empty() {
            let names: Vec<&str> = self.passed_over.iter().map(|b| backend_name(*b)).collect();
            write!(f, ", passing over {} (failed before on this driver)", names.join(" and "))?;
        }
        match self.fallback {
            Some(Fallback::Backend(b)) => {
                write!(f, ", a fallback ({}: no other adapter)", backend_name(b))?
            }
            Some(Fallback::Software) => write!(f, ", a fallback (a software adapter)")?,
            None => {}
        }
        Ok(())
    }
}

/// A backend that failed on a GPU and driver (the startup check, or the
/// area view's watch), kept in the settings: Automatic passes over the
/// adapters it names ([`pick_adapter`]). A new driver is tried again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failed {
    pub vendor: u32,
    pub device: u32,
    /// The driver's name and version ([`driver_of`]).
    pub driver: String,
    pub backend: Backend,
}

impl Failed {
    /// The failure of `info`'s adapter.
    pub fn of(info: &AdapterInfo) -> Failed {
        Failed {
            vendor: info.vendor,
            device: info.device,
            driver: driver_of(info),
            backend: info.backend,
        }
    }

    /// Whether `info` is the adapter that failed, on the same driver.
    pub fn names(&self, info: &AdapterInfo) -> bool {
        *self == Failed::of(info)
    }
}

/// An adapter's driver, as a failure is kept by: its name and its version
/// (wgpu's `driver` and `driver_info`; Direct3D 12 says the version alone,
/// Vulkan both). Metal says neither, nor the GPU's vendor and device (its
/// driver is the system's): the adapter's name and the system's version
/// stand for them, so that a record is of one GPU and ends with a system
/// update.
pub fn driver_of(info: &AdapterInfo) -> String {
    driver_on(info, system_version())
}

/// [`driver_of`], on a system of version `system`.
fn driver_on(info: &AdapterInfo, system: &str) -> String {
    let said = format!("{} {}", info.driver, info.driver_info).trim().to_string();
    if !said.is_empty() {
        return said;
    }
    format!("{} {system}", info.name).trim().to_string()
}

/// The operating system's version where a driver's stands for none
/// ([`driver_of`]): `macOS 15.1` on a Mac (its `SystemVersion.plist`),
/// nothing elsewhere (Vulkan and Direct3D 12 say their drivers).
fn system_version() -> &'static str {
    static VERSION: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    VERSION.get_or_init(|| {
        if Platform::current() != Platform::MacOs {
            return String::new();
        }
        std::fs::read_to_string("/System/Library/CoreServices/SystemVersion.plist")
            .ok()
            .and_then(|plist| product_version(&plist).map(|v| format!("macOS {v}")))
            .unwrap_or_default()
    })
}

/// `ProductVersion`'s string in a `SystemVersion.plist`.
fn product_version(plist: &str) -> Option<&str> {
    let after = plist.split_once("<key>ProductVersion</key>")?.1;
    let value = after.split_once("<string>")?.1.split_once("</string>")?.0.trim();
    (!value.is_empty() && value.len() <= 32).then_some(value)
}

/// The descriptor Moonglow's devices are opened with (the window's, the
/// startup check's, the headless one's), of an adapter with `features`
/// and `limits`: BC textures where it reads them (the game's are mostly
/// BC: uploaded as they are, else as RGBA8 four to eight times their
/// size) and the shadow volumes' depth as exact as the scene's
/// ([`crate::OPTIONAL_FEATURES`]); the GPU's memory taken in blocks of 8
/// to 64 MB, not wgpu's default 128 to 256 (a block is held while
/// anything is in it, and what the views and textures let go of stayed
/// held: 768 MB held for 270 used after Tyrants of the Moonsea's frame
/// test, 524 with these, no frame slower).
pub fn device_descriptor(
    label: &'static str,
    features: wgpu::Features,
    limits: wgpu::Limits,
) -> wgpu::DeviceDescriptor<'static> {
    wgpu::DeviceDescriptor {
        label: Some(label),
        required_features: features & crate::OPTIONAL_FEATURES,
        required_limits: limits,
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        ..Default::default()
    }
}

/// The backends `WGPU_BACKEND` names, as wgpu reads it (a list of names
/// with commas between, any case: `vulkan` or `vk`, `dx12` or `d3d12`,
/// `metal` or `mtl`, `gl`, `gles` or `opengl`): `None` where it is not
/// set, empty, or names none (wgpu's own reading then leaves no backend
/// at all: no adapter, and no window); and the names that are none.
pub fn named_backends(value: Option<&str>) -> (Option<wgpu::Backends>, Vec<String>) {
    let Some(value) = value else { return (None, Vec::new()) };
    let (mut backends, mut unknown) = (wgpu::Backends::empty(), Vec::new());
    for name in value.split(',').map(str::trim).filter(|n| !n.is_empty()) {
        backends |= match name.to_ascii_lowercase().as_str() {
            "vulkan" | "vk" => wgpu::Backends::VULKAN,
            "dx12" | "d3d12" => wgpu::Backends::DX12,
            "metal" | "mtl" => wgpu::Backends::METAL,
            "opengl" | "gles" | "gl" => wgpu::Backends::GL,
            _ => {
                unknown.push(name.to_string());
                continue;
            }
        };
    }
    ((!backends.is_empty()).then_some(backends), unknown)
}

/// [`named_backends`] of this run's `WGPU_BACKEND`: what the window's
/// instance, the headless one's and the pick all go by.
pub fn backends_from_env() -> (Option<wgpu::Backends>, Vec<String>) {
    named_backends(std::env::var("WGPU_BACKEND").ok().as_deref())
}

/// What to say of `WGPU_BACKEND`'s names that are no backend (`None`:
/// there are none).
pub fn unknown_backends_said(unknown: &[String], decides: bool) -> Option<String> {
    if unknown.is_empty() {
        return None;
    }
    let names: Vec<String> = unknown.iter().map(|n| format!("\"{n}\"")).collect();
    let rest = if decides { "the others it names decide" } else { "it is passed over" };
    Some(format!(
        "WGPU_BACKEND names no backend with {} (vulkan, dx12, metal and gl are the names): {rest}",
        names.join(", ")
    ))
}

/// wgpu's order of adapter kinds for a power preference (wgpu-core's
/// `request_adapter`): a kind that says what it is before Other (OpenGL's,
/// often), and software last.
fn kind_order(kind: DeviceType, power: PowerPreference) -> u8 {
    let integrated_first = power == PowerPreference::LowPower;
    match kind {
        DeviceType::DiscreteGpu if integrated_first => 2,
        DeviceType::IntegratedGpu if integrated_first => 1,
        DeviceType::DiscreteGpu => 1,
        DeviceType::IntegratedGpu => 2,
        DeviceType::Other => 3,
        DeviceType::VirtualGpu => 4,
        DeviceType::Cpu => 5,
    }
}

/// The indices of `adapters` (or of those `keep` keeps) in wgpu's order
/// of preference: as listed (by backend, as wgpu enumerates them), sorted
/// by kind for `power` (none: as listed).
fn wgpu_order(
    adapters: &[AdapterInfo],
    power: PowerPreference,
    keep: impl Fn(&AdapterInfo) -> bool,
) -> Vec<usize> {
    let mut order: Vec<usize> = (0..adapters.len()).filter(|&i| keep(&adapters[i])).collect();
    if power != PowerPreference::None {
        // (Stable: the backends' order within a kind.)
        order.sort_by_key(|&i| kind_order(adapters[i].device_type, power));
    }
    order
}

/// The adapter to draw with, of `adapters` (as wgpu enumerated them:
/// those that can draw to the window, if there is one). `wanted`: the
/// backend chosen in Options (`None`: Automatic). `from_environment`:
/// `WGPU_BACKEND` is set, and the adapters are its backends': it decides,
/// over `wanted`, in wgpu's order. A backend chosen with no adapter falls
/// back to Automatic (and says so in [`Pick::missing`]). Automatic takes
/// an adapter of the platform's backends that is no software one, passing
/// over those `failed` names (each a backend that failed on that GPU and
/// driver: [`Pick::passed_over`]) unless that leaves none; OpenGL or a
/// software adapter only where there is no other, and it says so
/// ([`Pick::fallback`]). `None` only without any adapter.
pub fn pick_adapter(
    adapters: &[AdapterInfo],
    wanted: Option<Backend>,
    from_environment: bool,
    power: PowerPreference,
    platform: Platform,
    failed: &[Failed],
) -> Option<Pick> {
    let pick = |index: usize, why, missing| Pick {
        index,
        why,
        missing,
        passed_over: Vec::new(),
        fallback: Fallback::of(&adapters[index], platform),
    };
    if from_environment {
        let index = *wgpu_order(adapters, power, |_| true).first()?;
        return Some(pick(index, Why::Environment, None));
    }
    let mut missing = None;
    if let Some(backend) = wanted {
        match wgpu_order(adapters, power, |a| a.backend == backend).first() {
            Some(&index) => return Some(pick(index, Why::Chosen(backend), None)),
            None => missing = Some(backend),
        }
    }
    let recorded = |a: &AdapterInfo| failed.iter().any(|f| f.names(a));
    // (An adapter Automatic draws with: not OpenGL, nor a software one.)
    let worth = |a: &AdapterInfo| Fallback::of(a, platform).is_none();
    // Those that didn't fail before; else (every one recorded) as though
    // none were; else (none to draw with at all) whatever is there.
    let unrecorded = wgpu_order(adapters, power, |a| worth(a) && !recorded(a));
    let skip = !unrecorded.is_empty();
    let any_worth = adapters.iter().any(worth);
    let kept = |a: &AdapterInfo| if any_worth { worth(a) && !(skip && recorded(a)) } else { true };
    let mut passed_over: Vec<Backend> = Vec::new();
    for a in adapters.iter().filter(|a| skip && recorded(a)) {
        if !passed_over.contains(&a.backend) {
            passed_over.push(a.backend);
        }
    }
    let first = *wgpu_order(adapters, power, kept).first()?;
    let taken = &adapters[first];
    let (mut index, mut why) = (first, Why::Automatic);
    if platform == Platform::Windows && taken.vendor == AMD && taken.backend == Backend::Vulkan {
        // The same GPU through Direct3D 12: by its device ID, or (one of
        // them not saying it) the same kind of AMD GPU.
        let dx12 = |a: &AdapterInfo| a.backend == Backend::Dx12 && a.vendor == AMD && kept(a);
        let same = wgpu_order(adapters, power, |a| dx12(a) && a.device == taken.device);
        let like = wgpu_order(adapters, power, |a| dx12(a) && a.device_type == taken.device_type);
        if let Some(&i) = same.first().or(like.first()) {
            (index, why) = (i, Why::AmdOnWindows);
        }
    }
    Some(Pick { passed_over, ..pick(index, why, missing) })
}

/// The same GPU as `adapters[taken]` through another of `platform`'s
/// backends (in their order), that `failed` doesn't name: the one to take
/// where `taken` fails the startup check. By device ID, or (one of them
/// not saying it) the same vendor and kind of GPU.
pub fn other_backend(
    adapters: &[AdapterInfo],
    taken: usize,
    platform: Platform,
    failed: &[Failed],
) -> Option<usize> {
    let t = adapters.get(taken)?;
    let same = |a: &AdapterInfo| {
        a.vendor == t.vendor
            && (a.device == t.device
                || ((a.device == 0 || t.device == 0) && a.device_type == t.device_type))
    };
    platform.backends().iter().filter(|&&b| b != t.backend).find_map(|&b| {
        adapters
            .iter()
            .position(|a| a.backend == b && same(a) && !failed.iter().any(|f| f.names(a)))
    })
}

/// What [`start`] made of the startup check: the adapter taken, why
/// ([`pick_adapter`]'s pick), each adapter checked with what it came to,
/// and whether it took another backend for one that failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Started {
    pub index: usize,
    pub pick: Pick,
    /// Each adapter checked (its index) and what failed, if it did.
    pub checks: Vec<(usize, Result<(), String>)>,
    /// The pick failed and the same GPU's other backend was taken.
    pub switched: bool,
}

impl Started {
    /// The adapters that failed the check, to be recorded ([`Failed`]).
    pub fn failures(&self) -> impl Iterator<Item = (usize, &str)> {
        self.checks.iter().filter_map(|(i, r)| r.as_ref().err().map(|e| (*i, e.as_str())))
    }

    /// Whether the adapter drawn with failed the check: kept, as it was
    /// asked for or as no other passed; the user is told.
    pub fn failing(&self) -> bool {
        self.checks.iter().any(|(i, r)| *i == self.index && r.is_err())
    }
}

/// The window's adapter, checked as Moonglow starts: [`pick_adapter`]'s
/// pick, then `check` (the startup check, [`crate::probe::probe`], on that
/// adapter: `Err` saying what it drew wrong). Where it fails on Automatic
/// (not a backend asked for in Options or by `WGPU_BACKEND`, which is kept
/// whatever it draws), the same GPU's other backend ([`other_backend`]) is
/// checked and taken if it passes; once (at most one switch a start). If
/// that fails too, the pick is kept. `None` only without any adapter.
#[allow(clippy::too_many_arguments)]
pub fn start(
    adapters: &[AdapterInfo],
    wanted: Option<Backend>,
    from_environment: bool,
    power: PowerPreference,
    platform: Platform,
    failed: &[Failed],
    mut check: impl FnMut(usize) -> Result<(), String>,
) -> Option<Started> {
    let pick = pick_adapter(adapters, wanted, from_environment, power, platform, failed)?;
    let first = check(pick.index);
    let mut started =
        Started { index: pick.index, checks: vec![(pick.index, first)], pick, switched: false };
    if !started.failing() || started.pick.asked_for() {
        return Some(started);
    }
    let mut known = failed.to_vec();
    known.push(Failed::of(&adapters[started.index]));
    if let Some(other) = other_backend(adapters, started.index, platform, &known) {
        let outcome = check(other);
        let passed = outcome.is_ok();
        started.checks.push((other, outcome));
        if passed {
            started.index = other;
            started.switched = true;
        }
    }
    Some(started)
}

impl fmt::Display for Started {
    /// For the debug log's `graphics:` line: the pick, and what the check
    /// made of each adapter.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.pick)?;
        for (i, (_, outcome)) in self.checks.iter().enumerate() {
            let what = if i == 0 { "the startup check" } else { "then" };
            match outcome {
                Ok(()) => write!(f, "; {what}: drawn right")?,
                Err(why) => write!(f, "; {what}: failed ({why})")?,
            }
        }
        if self.switched {
            write!(f, ", so the other backend was taken")?;
        } else if self.failing() && self.pick.asked_for() {
            write!(f, ", kept as asked for")?;
        } else if self.failing() {
            write!(f, ", kept: no other backend drew right")?;
        }
        Ok(())
    }
}

/// The adapters, for the debug log: each one's backend, name and kind.
pub fn describe_adapters(adapters: &[AdapterInfo]) -> String {
    let each: Vec<String> = adapters
        .iter()
        .map(|a| {
            format!(
                "{} {} ({:?}, vendor {:#06x}, device {:#06x})",
                backend_name(a.backend),
                a.name,
                a.device_type,
                a.vendor,
                a.device
            )
        })
        .collect();
    if each.is_empty() { "none".into() } else { each.join("; ") }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NVIDIA: u32 = 0x10de;
    const INTEL: u32 = 0x8086;
    const HIGH: PowerPreference = PowerPreference::HighPerformance;

    fn adapter(backend: Backend, vendor: u32, device: u32, kind: DeviceType) -> AdapterInfo {
        AdapterInfo { vendor, device, ..AdapterInfo::new(kind, backend) }
    }

    /// What Windows lists for one GPU, in wgpu's order of backends:
    /// Vulkan, Direct3D 12 (with Microsoft's software adapter), OpenGL.
    fn windows(vendor: u32, device: u32, kind: DeviceType) -> Vec<AdapterInfo> {
        vec![
            adapter(Backend::Vulkan, vendor, device, kind),
            adapter(Backend::Dx12, vendor, device, kind),
            adapter(Backend::Dx12, 0x1414, 0x8c, DeviceType::Cpu),
            adapter(Backend::Gl, vendor, 0, DeviceType::Other),
        ]
    }

    fn pick(adapters: &[AdapterInfo], wanted: Option<Backend>, platform: Platform) -> Pick {
        pick_adapter(adapters, wanted, false, HIGH, platform, &[]).unwrap()
    }

    /// A pick of nothing passed over, no fallback.
    fn picked(index: usize, why: Why, missing: Option<Backend>) -> Pick {
        Pick { index, why, missing, passed_over: Vec::new(), fallback: None }
    }

    /// El Grillo's Radeon PRO (integrated, AMD's Vulkan driver): Direct3D
    /// 12 on Automatic.
    #[test]
    fn amd_on_windows_takes_direct3d_12_over_vulkan() {
        let list = windows(AMD, 0x1638, DeviceType::IntegratedGpu);
        let p = pick(&list, None, Platform::Windows);
        assert_eq!(p, picked(1, Why::AmdOnWindows, None));
        assert_eq!(list[p.index].backend, Backend::Dx12);
        // (One of them not saying its device: the same kind of AMD GPU.)
        let mut list = list;
        list[1].device = 0;
        assert_eq!(pick(&list, None, Platform::Windows).index, 1);
        // Without a Direct3D 12 adapter, Vulkan all the same.
        let vulkan_only = &list[..1];
        assert_eq!(pick(vulkan_only, None, Platform::Windows).why, Why::Automatic);
    }

    #[test]
    fn nvidia_and_intel_on_windows_keep_wgpus_order() {
        for (vendor, kind) in
            [(NVIDIA, DeviceType::DiscreteGpu), (INTEL, DeviceType::IntegratedGpu)]
        {
            let list = windows(vendor, 0x2204, kind);
            let p = pick(&list, None, Platform::Windows);
            assert_eq!(p, picked(0, Why::Automatic, None));
            assert_eq!(list[p.index].backend, Backend::Vulkan);
        }
    }

    #[test]
    fn amd_on_linux_keeps_vulkan() {
        let list = vec![
            adapter(Backend::Vulkan, AMD, 0x7550, DeviceType::DiscreteGpu),
            adapter(Backend::Gl, AMD, 0, DeviceType::Other),
        ];
        let p = pick(&list, None, Platform::Linux);
        assert_eq!(p, picked(0, Why::Automatic, None));
        // (Nor would a Direct3D 12 adapter there change it: Windows only.)
        let mut odd = list.clone();
        odd.insert(1, adapter(Backend::Dx12, AMD, 0x7550, DeviceType::DiscreteGpu));
        assert_eq!(pick(&odd, None, Platform::Linux).index, 0);
    }

    #[test]
    fn a_backend_chosen_wins() {
        let amd = windows(AMD, 0x1638, DeviceType::IntegratedGpu);
        let p = pick(&amd, Some(Backend::Vulkan), Platform::Windows);
        assert_eq!(p, picked(0, Why::Chosen(Backend::Vulkan), None));
        let nvidia = windows(NVIDIA, 0x2204, DeviceType::DiscreteGpu);
        assert_eq!(pick(&nvidia, Some(Backend::Dx12), Platform::Windows).index, 1);
        assert_eq!(pick(&nvidia, Some(Backend::Vulkan), Platform::Windows).index, 0);
    }

    /// `WGPU_BACKEND` decides over the choice and over the AMD preference:
    /// wgpu's order among the backends it named.
    #[test]
    fn wgpu_backend_decides_over_the_choice() {
        let amd = windows(AMD, 0x1638, DeviceType::IntegratedGpu);
        let p = pick_adapter(&amd, Some(Backend::Gl), true, HIGH, Platform::Windows, &[]).unwrap();
        assert_eq!(p, picked(0, Why::Environment, None));
    }

    /// A backend chosen with no adapter (Direct3D 12 on Linux, Metal from
    /// a settings file brought from a Mac): Automatic, and it says so.
    #[test]
    fn a_backend_chosen_with_no_adapter_falls_back_to_automatic() {
        let linux = vec![adapter(Backend::Vulkan, INTEL, 0x46a6, DeviceType::IntegratedGpu)];
        let p = pick(&linux, Some(Backend::Dx12), Platform::Linux);
        assert_eq!(p, picked(0, Why::Automatic, Some(Backend::Dx12)));
        assert_eq!(
            p.to_string(),
            "no Direct3D 12 adapter for the backend chosen, so Automatic (wgpu's order)"
        );
        // On Windows with AMD, the fallback is Automatic's: Direct3D 12.
        let amd = windows(AMD, 0x1638, DeviceType::IntegratedGpu);
        let p = pick(&amd, Some(Backend::Metal), Platform::Windows);
        assert_eq!(p, picked(1, Why::AmdOnWindows, Some(Backend::Metal)));
        // No adapter at all: none.
        assert_eq!(pick_adapter(&[], None, false, HIGH, Platform::Windows, &[]), None);
        assert_eq!(
            pick_adapter(&[], Some(Backend::Vulkan), true, HIGH, Platform::Linux, &[]),
            None
        );
    }

    /// A laptop's integrated and discrete GPUs: the discrete one for high
    /// performance, the integrated one for low power, as wgpu chose them
    /// (and, AMD's on Windows, through Direct3D 12).
    #[test]
    fn integrated_and_discrete_gpus_are_picked_by_power_preference() {
        let laptop = vec![
            adapter(Backend::Vulkan, INTEL, 0x46a6, DeviceType::IntegratedGpu),
            adapter(Backend::Vulkan, NVIDIA, 0x2520, DeviceType::DiscreteGpu),
            adapter(Backend::Gl, INTEL, 0, DeviceType::Other),
        ];
        let at = |list: &[AdapterInfo], power, platform| {
            pick_adapter(list, None, false, power, platform, &[]).unwrap().index
        };
        assert_eq!(at(&laptop, HIGH, Platform::Linux), 1);
        assert_eq!(at(&laptop, PowerPreference::LowPower, Platform::Linux), 0);
        assert_eq!(at(&laptop, PowerPreference::None, Platform::Linux), 0);
        // An AMD integrated GPU beside an NVIDIA discrete one on Windows:
        // the discrete one, through Vulkan as before; for low power, the
        // AMD one, through Direct3D 12.
        let mixed = vec![
            adapter(Backend::Vulkan, AMD, 0x1638, DeviceType::IntegratedGpu),
            adapter(Backend::Vulkan, NVIDIA, 0x2520, DeviceType::DiscreteGpu),
            adapter(Backend::Dx12, NVIDIA, 0x2520, DeviceType::DiscreteGpu),
            adapter(Backend::Dx12, AMD, 0x1638, DeviceType::IntegratedGpu),
        ];
        assert_eq!(at(&mixed, HIGH, Platform::Windows), 1);
        let low =
            pick_adapter(&mixed, None, false, PowerPreference::LowPower, Platform::Windows, &[]);
        assert_eq!(low.unwrap(), picked(3, Why::AmdOnWindows, None));
        // Two AMD GPUs: the discrete one's own Direct3D 12 adapter, not the
        // integrated one's.
        let two = vec![
            adapter(Backend::Vulkan, AMD, 0x1638, DeviceType::IntegratedGpu),
            adapter(Backend::Vulkan, AMD, 0x7550, DeviceType::DiscreteGpu),
            adapter(Backend::Dx12, AMD, 0x1638, DeviceType::IntegratedGpu),
            adapter(Backend::Dx12, AMD, 0x7550, DeviceType::DiscreteGpu),
        ];
        assert_eq!(at(&two, HIGH, Platform::Windows), 3);
    }

    #[test]
    fn each_platform_offers_its_own_backends() {
        use Backend::*;
        assert_eq!(Platform::Windows.backends(), [Vulkan, Dx12]);
        assert_eq!(Platform::Linux.backends(), [Vulkan]);
        assert_eq!(Platform::MacOs.backends(), [Metal]);
        assert!(!Platform::current().backends().is_empty());
    }

    /// What Windows lists for an AMD GPU with its driver's version.
    fn amd_windows(driver: &str) -> Vec<AdapterInfo> {
        let mut list = windows(AMD, 0x67df, DeviceType::DiscreteGpu);
        for a in &mut list {
            a.driver_info = driver.into();
        }
        list
    }

    /// The check, failing on the backends given.
    fn failing_on(
        list: &[AdapterInfo],
        broken: &'static [Backend],
    ) -> impl Fn(usize) -> Result<(), String> {
        let backends: Vec<Backend> = list.iter().map(|a| a.backend).collect();
        move |i: usize| {
            if broken.contains(&backends[i]) { Err("nothing drawn".into()) } else { Ok(()) }
        }
    }

    /// [`start`] on Windows, and the adapters checked, in order.
    fn started(
        list: &[AdapterInfo],
        wanted: Option<Backend>,
        failed: &[Failed],
        broken: &'static [Backend],
    ) -> (Started, Vec<usize>) {
        let check = failing_on(list, broken);
        let mut asked = Vec::new();
        let counted = |i| {
            asked.push(i);
            check(i)
        };
        let s = start(list, wanted, false, HIGH, Platform::Windows, failed, counted).unwrap();
        (s, asked)
    }

    /// Heed's RX 580: Direct3D 12 (Automatic's choice for AMD on Windows)
    /// draws nothing, so the check takes Vulkan before the window opens.
    /// El Grillo's Radeon PRO draws its first frames right through Vulkan
    /// and loses its tiles later: the check passes Direct3D 12 and it is
    /// kept, as Automatic chose it.
    #[test]
    fn the_startup_check_takes_the_other_backend_where_the_pick_draws_nothing() {
        let list = amd_windows("25.9.1");
        let (heed, asked) = started(&list, None, &[], &[Backend::Dx12]);
        assert_eq!(asked, [1, 0], "Direct3D 12 checked first, then Vulkan");
        assert_eq!((heed.index, heed.switched, heed.failing()), (0, true, false));
        assert_eq!(heed.failures().collect::<Vec<_>>(), [(1, "nothing drawn")]);
        assert_eq!(
            heed.to_string(),
            "Automatic (Direct3D 12 rather than Vulkan for AMD on Windows); the startup check: \
             failed (nothing drawn); then: drawn right, so the other backend was taken"
        );
        let (grillo, asked) = started(&list, None, &[], &[Backend::Vulkan]);
        assert_eq!(asked, [1], "only the pick is checked where it draws right");
        assert_eq!((grillo.index, grillo.switched, grillo.failing()), (1, false, false));
        assert_eq!(grillo.failures().count(), 0);
    }

    /// A backend chosen in Options (or by `WGPU_BACKEND`) is kept though it
    /// fails, and nothing else is tried; nor when both backends fail: the
    /// pick is kept, once each checked (one switch at most).
    #[test]
    fn a_backend_asked_for_or_every_backend_failing_is_kept() {
        let list = amd_windows("25.9.1");
        let (chosen, asked) = started(&list, Some(Backend::Dx12), &[], &[Backend::Dx12]);
        assert_eq!((asked, chosen.index, chosen.failing()), (vec![1], 1, true));
        assert!(chosen.to_string().ends_with(", kept as asked for"), "{chosen}");
        let check = failing_on(&list, &[Backend::Vulkan]);
        let env = start(&list, Some(Backend::Dx12), true, HIGH, Platform::Windows, &[], check);
        let env = env.unwrap();
        assert_eq!((env.index, env.failing(), env.switched), (0, true, false));
        let (both, asked) = started(&list, None, &[], &[Backend::Dx12, Backend::Vulkan]);
        assert_eq!(asked, [1, 0]);
        assert_eq!((both.index, both.switched, both.failing()), (1, false, true));
        assert_eq!(both.failures().count(), 2);
        assert!(both.to_string().ends_with("kept: no other backend drew right"), "{both}");
        // (Linux: no other backend to take.)
        let linux = vec![adapter(Backend::Vulkan, AMD, 0x67df, DeviceType::DiscreteGpu)];
        let check = failing_on(&linux, &[Backend::Vulkan]);
        let s = start(&linux, None, false, HIGH, Platform::Linux, &[], check).unwrap();
        assert_eq!((s.index, s.checks.len(), s.failing()), (0, 1, true));
    }

    /// A recorded failure (this GPU, this driver, this backend) is passed
    /// over by Automatic on the next start, without checking it; a driver
    /// update tries it again; a backend chosen in Options is taken all the
    /// same; and with every backend recorded, none is passed over.
    #[test]
    fn automatic_passes_over_a_recorded_failure_until_the_driver_changes() {
        let list = amd_windows("25.9.1");
        let recorded = [Failed::of(&list[1])];
        assert_eq!(recorded[0].driver, "25.9.1", "Direct3D 12 says its version alone");
        let p = pick_adapter(&list, None, false, HIGH, Platform::Windows, &recorded).unwrap();
        assert_eq!((p.index, p.why), (0, Why::Automatic));
        assert_eq!(p.passed_over, [Backend::Dx12]);
        assert!(p.to_string().ends_with("passing over Direct3D 12 (failed before on this driver)"));
        let (s, asked) = started(&list, None, &recorded, &[]);
        assert_eq!((asked, s.index), (vec![0], 0));
        // A new driver: Direct3D 12 again.
        let updated = amd_windows("26.1.1");
        let p = pick_adapter(&updated, None, false, HIGH, Platform::Windows, &recorded);
        assert_eq!(p.unwrap(), picked(1, Why::AmdOnWindows, None));
        // Chosen in Options: taken.
        let p = pick_adapter(&list, Some(Backend::Dx12), false, HIGH, Platform::Windows, &recorded);
        assert_eq!(p.unwrap().index, 1);
        // Every adapter recorded: as though none were.
        let all: Vec<Failed> = list.iter().map(Failed::of).collect();
        let p = pick_adapter(&list, None, false, HIGH, Platform::Windows, &all).unwrap();
        assert_eq!(p, picked(1, Why::AmdOnWindows, None));
        // Both of the GPU's backends recorded (both failed the check one
        // start): not OpenGL, nor Microsoft's software adapter, which are
        // left; as though none were.
        let both = [Failed::of(&list[0]), Failed::of(&list[1])];
        let p = pick_adapter(&list, None, false, HIGH, Platform::Windows, &both).unwrap();
        assert_eq!(p, picked(1, Why::AmdOnWindows, None));
        // Another GPU left: taken.
        let mut two = list.clone();
        two.push(adapter(Backend::Vulkan, NVIDIA, 0x2520, DeviceType::DiscreteGpu));
        let p = pick_adapter(&two, None, false, HIGH, Platform::Windows, &both).unwrap();
        assert_eq!((p.index, p.why), (4, Why::Automatic));
    }

    /// Automatic takes OpenGL or a software adapter only where nothing
    /// else is there, and says so; with a GPU's backend recorded, the
    /// other GPU, not the first GPU's OpenGL (a Linux laptop asked for low
    /// power, its Intel GPU's Vulkan recorded: OpenGL on Intel came first
    /// by kind, against the rule that OpenGL stands in for nothing).
    #[test]
    fn automatic_takes_a_fallback_only_where_nothing_else_draws() {
        let laptop = vec![
            adapter(Backend::Vulkan, INTEL, 0x46a6, DeviceType::IntegratedGpu),
            adapter(Backend::Vulkan, NVIDIA, 0x2520, DeviceType::DiscreteGpu),
            adapter(Backend::Vulkan, 0x10005, 0, DeviceType::Cpu),
            adapter(Backend::Gl, INTEL, 0, DeviceType::IntegratedGpu),
        ];
        let low = PowerPreference::LowPower;
        let recorded = [Failed::of(&laptop[0])];
        let p = pick_adapter(&laptop, None, false, low, Platform::Linux, &recorded).unwrap();
        assert_eq!((p.index, p.why, p.fallback), (1, Why::Automatic, None));
        assert_eq!(p.passed_over, [Backend::Vulkan]);
        // (Nor OpenGL before a GPU's Vulkan with nothing recorded.)
        let gl_first = vec![laptop[3].clone(), laptop[1].clone()];
        let p = pick_adapter(&gl_first, None, false, low, Platform::Linux, &[]).unwrap();
        assert_eq!((p.index, p.fallback), (1, None));
        // No Vulkan driver: OpenGL, as a fallback.
        let gl = vec![adapter(Backend::Gl, INTEL, 0, DeviceType::IntegratedGpu)];
        let p = pick(&gl, None, Platform::Linux);
        assert_eq!((p.index, p.fallback), (0, Some(Fallback::Backend(Backend::Gl))));
        assert_eq!(
            p.to_string(),
            "Automatic (wgpu's order), a fallback (OpenGL: no other adapter)"
        );
        // The basic display driver, a remote desktop: Microsoft's software
        // adapter, as a fallback; so llvmpipe alone on Linux.
        let warp = vec![adapter(Backend::Dx12, 0x1414, 0x8c, DeviceType::Cpu)];
        let p = pick(&warp, None, Platform::Windows);
        assert_eq!((p.index, p.fallback), (0, Some(Fallback::Software)));
        let p = pick(&laptop[2..3], None, Platform::Linux);
        assert_eq!(p.fallback, Some(Fallback::Software));
        // A software adapter and OpenGL: the software one of the
        // platform's backend last, as wgpu orders kinds.
        let p = pick(&laptop[2..], None, Platform::Linux);
        assert_eq!((p.index, p.fallback), (1, Some(Fallback::Backend(Backend::Gl))));
        // Asked for: said all the same.
        let p = pick(&laptop, Some(Backend::Gl), Platform::Linux);
        assert_eq!((p.index, p.fallback), (3, Some(Fallback::Backend(Backend::Gl))));
        // What the builder is told names the backend to install.
        let said = Fallback::Backend(Backend::Gl).said(Platform::Linux);
        assert!(said.contains("through OpenGL") && said.contains("Vulkan driver"), "{said}");
        let said = Fallback::Software.said(Platform::Windows);
        assert!(said.contains("Vulkan or Direct3D 12"), "{said}");
    }

    /// Metal says no driver, vendor or device: a record there is of the
    /// GPU by its name, and of the system's version. Where the driver is
    /// said, it alone.
    #[test]
    fn a_metal_record_names_the_gpu_and_the_system() {
        let metal = |name: &str| {
            let mut a = adapter(Backend::Metal, 0, 0, DeviceType::IntegratedGpu);
            a.name = name.into();
            a
        };
        let (m1, pro) = (metal("Apple M1"), metal("AMD Radeon Pro 5500M"));
        assert_ne!(Failed::of(&m1), Failed::of(&pro));
        assert!(Failed::of(&m1).driver.starts_with("Apple M1"));
        assert_eq!(driver_on(&m1, "macOS 15.1"), "Apple M1 macOS 15.1");
        assert_ne!(driver_on(&m1, "macOS 15.1"), driver_on(&m1, "macOS 15.2"));
        assert_eq!(driver_on(&m1, ""), "Apple M1");
        let mut vulkan = adapter(Backend::Vulkan, AMD, 0x7550, DeviceType::DiscreteGpu);
        (vulkan.name, vulkan.driver, vulkan.driver_info) =
            ("RX 9070".into(), "radv".into(), "Mesa 26.0".into());
        assert_eq!(driver_on(&vulkan, "macOS 15.1"), "radv Mesa 26.0");
        let plist = "<dict>\n\t<key>ProductName</key>\n\t<string>macOS</string>\n\t\
                     <key>ProductVersion</key>\n\t<string>15.1</string>\n</dict>";
        assert_eq!(product_version(plist), Some("15.1"));
        assert_eq!(product_version("<key>ProductVersion</key><string>"), None);
        assert_eq!(product_version(""), None);
    }

    /// `WGPU_BACKEND` set but empty, or naming no backend, is as though it
    /// were not set (wgpu's own reading leaves no backend at all then: no
    /// adapter, and `mg`'s pictures said there was no GPU); a name that is
    /// none among others is said.
    #[test]
    fn wgpu_backend_naming_no_backend_is_as_unset() {
        let none: Vec<String> = Vec::new();
        assert_eq!(named_backends(None), (None, none.clone()));
        assert_eq!(named_backends(Some("")), (None, none.clone()));
        assert_eq!(named_backends(Some(" , ")), (None, none.clone()));
        assert_eq!(named_backends(Some("d3d")), (None, vec!["d3d".to_string()]));
        let (some, unknown) = named_backends(Some("vulkan,nope"));
        assert_eq!((some, unknown), (Some(wgpu::Backends::VULKAN), vec!["nope".to_string()]));
        let (both, unknown) = named_backends(Some(" DX12, gl "));
        assert_eq!((both, unknown), (Some(wgpu::Backends::DX12 | wgpu::Backends::GL), none));
        // (As wgpu reads the names it knows.)
        for names in ["vulkan", "vk,metal", "D3D12", "opengl,gles", "mtl"] {
            assert_eq!(named_backends(Some(names)).0, Some(wgpu::Backends::from_comma_list(names)));
        }
        assert_eq!(unknown_backends_said(&[], true), None);
        let said = unknown_backends_said(&["d3d".into()], false).unwrap();
        assert!(said.contains("\"d3d\"") && said.ends_with("it is passed over"), "{said}");
    }

    /// The other backend is the same GPU's: not the software adapter, nor
    /// another GPU, nor one recorded.
    #[test]
    fn the_other_backend_is_the_same_gpus() {
        let list = windows(AMD, 0x67df, DeviceType::DiscreteGpu);
        assert_eq!(other_backend(&list, 1, Platform::Windows, &[]), Some(0));
        assert_eq!(other_backend(&list, 0, Platform::Windows, &[]), Some(1));
        assert_eq!(other_backend(&list, 0, Platform::Windows, &[Failed::of(&list[1])]), None);
        assert_eq!(other_backend(&list, 0, Platform::Linux, &[]), None);
        let two = vec![
            adapter(Backend::Vulkan, NVIDIA, 0x2520, DeviceType::DiscreteGpu),
            adapter(Backend::Dx12, AMD, 0x1638, DeviceType::IntegratedGpu),
            adapter(Backend::Dx12, NVIDIA, 0x2520, DeviceType::DiscreteGpu),
        ];
        assert_eq!(other_backend(&two, 0, Platform::Windows, &[]), Some(2));
        assert_eq!(other_backend(&two, 1, Platform::Windows, &[]), None);
    }
}
