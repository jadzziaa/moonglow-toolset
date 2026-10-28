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
//! through Direct3D 12 drew them right.

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

/// The adapter taken: its index among those given, why, and the backend
/// chosen in Options that had no adapter (Automatic was used instead).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pick {
    pub index: usize,
    pub why: Why,
    pub missing: Option<Backend>,
}

impl fmt::Display for Pick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(missing) = self.missing {
            write!(f, "no {} adapter for the backend chosen, so ", backend_name(missing))?;
        }
        match self.why {
            Why::Chosen(b) => write!(f, "{} chosen in Options", backend_name(b)),
            Why::Environment => write!(f, "WGPU_BACKEND"),
            Why::Automatic => write!(f, "Automatic (wgpu's order)"),
            Why::AmdOnWindows => {
                write!(f, "Automatic (Direct3D 12 rather than Vulkan for AMD on Windows)")
            }
        }
    }
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
/// back to Automatic (and says so in [`Pick::missing`]). `None` only
/// without any adapter.
pub fn pick_adapter(
    adapters: &[AdapterInfo],
    wanted: Option<Backend>,
    from_environment: bool,
    power: PowerPreference,
    platform: Platform,
) -> Option<Pick> {
    if from_environment {
        let index = *wgpu_order(adapters, power, |_| true).first()?;
        return Some(Pick { index, why: Why::Environment, missing: None });
    }
    let mut missing = None;
    if let Some(backend) = wanted {
        match wgpu_order(adapters, power, |a| a.backend == backend).first() {
            Some(&index) => return Some(Pick { index, why: Why::Chosen(backend), missing: None }),
            None => missing = Some(backend),
        }
    }
    let first = *wgpu_order(adapters, power, |_| true).first()?;
    let taken = &adapters[first];
    if platform == Platform::Windows && taken.vendor == AMD && taken.backend == Backend::Vulkan {
        // The same GPU through Direct3D 12: by its device ID, or (one of
        // them not saying it) the same kind of AMD GPU.
        let dx12 = |a: &AdapterInfo| a.backend == Backend::Dx12 && a.vendor == AMD;
        let same = wgpu_order(adapters, power, |a| dx12(a) && a.device == taken.device);
        let like = wgpu_order(adapters, power, |a| dx12(a) && a.device_type == taken.device_type);
        if let Some(&index) = same.first().or(like.first()) {
            return Some(Pick { index, why: Why::AmdOnWindows, missing });
        }
    }
    Some(Pick { index: first, why: Why::Automatic, missing })
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
        pick_adapter(adapters, wanted, false, HIGH, platform).unwrap()
    }

    /// El Grillo's Radeon PRO (integrated, AMD's Vulkan driver): Direct3D
    /// 12 on Automatic.
    #[test]
    fn amd_on_windows_takes_direct3d_12_over_vulkan() {
        let list = windows(AMD, 0x1638, DeviceType::IntegratedGpu);
        let p = pick(&list, None, Platform::Windows);
        assert_eq!(p, Pick { index: 1, why: Why::AmdOnWindows, missing: None });
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
            assert_eq!(p, Pick { index: 0, why: Why::Automatic, missing: None });
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
        assert_eq!(p, Pick { index: 0, why: Why::Automatic, missing: None });
        // (Nor would a Direct3D 12 adapter there change it: Windows only.)
        let mut odd = list.clone();
        odd.insert(1, adapter(Backend::Dx12, AMD, 0x7550, DeviceType::DiscreteGpu));
        assert_eq!(pick(&odd, None, Platform::Linux).index, 0);
    }

    #[test]
    fn a_backend_chosen_wins() {
        let amd = windows(AMD, 0x1638, DeviceType::IntegratedGpu);
        let p = pick(&amd, Some(Backend::Vulkan), Platform::Windows);
        assert_eq!(p, Pick { index: 0, why: Why::Chosen(Backend::Vulkan), missing: None });
        let nvidia = windows(NVIDIA, 0x2204, DeviceType::DiscreteGpu);
        assert_eq!(pick(&nvidia, Some(Backend::Dx12), Platform::Windows).index, 1);
        assert_eq!(pick(&nvidia, Some(Backend::Vulkan), Platform::Windows).index, 0);
    }

    /// `WGPU_BACKEND` decides over the choice and over the AMD preference:
    /// wgpu's order among the backends it named.
    #[test]
    fn wgpu_backend_decides_over_the_choice() {
        let amd = windows(AMD, 0x1638, DeviceType::IntegratedGpu);
        let p = pick_adapter(&amd, Some(Backend::Gl), true, HIGH, Platform::Windows).unwrap();
        assert_eq!(p, Pick { index: 0, why: Why::Environment, missing: None });
    }

    /// A backend chosen with no adapter (Direct3D 12 on Linux, Metal from
    /// a settings file brought from a Mac): Automatic, and it says so.
    #[test]
    fn a_backend_chosen_with_no_adapter_falls_back_to_automatic() {
        let linux = vec![adapter(Backend::Vulkan, INTEL, 0x46a6, DeviceType::IntegratedGpu)];
        let p = pick(&linux, Some(Backend::Dx12), Platform::Linux);
        assert_eq!(p, Pick { index: 0, why: Why::Automatic, missing: Some(Backend::Dx12) });
        assert_eq!(
            p.to_string(),
            "no Direct3D 12 adapter for the backend chosen, so Automatic (wgpu's order)"
        );
        // On Windows with AMD, the fallback is Automatic's: Direct3D 12.
        let amd = windows(AMD, 0x1638, DeviceType::IntegratedGpu);
        let p = pick(&amd, Some(Backend::Metal), Platform::Windows);
        assert_eq!(p, Pick { index: 1, why: Why::AmdOnWindows, missing: Some(Backend::Metal) });
        // No adapter at all: none.
        assert_eq!(pick_adapter(&[], None, false, HIGH, Platform::Windows), None);
        assert_eq!(pick_adapter(&[], Some(Backend::Vulkan), true, HIGH, Platform::Linux), None);
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
            pick_adapter(list, None, false, power, platform).unwrap().index
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
        let low = pick_adapter(&mixed, None, false, PowerPreference::LowPower, Platform::Windows);
        assert_eq!(low.unwrap(), Pick { index: 3, why: Why::AmdOnWindows, missing: None });
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
}
