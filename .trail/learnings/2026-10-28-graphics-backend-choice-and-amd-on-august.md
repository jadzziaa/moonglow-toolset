---
title: Graphics backend choice and AMD on Windows
author: August
date: 2026-10-28
branch: graphics-backend
cost: { minutes: 1352, tokens: 11794431 }
touches:
  - crates/mg-render/src/backend.rs
  - apps/moonglow/src/main.rs
  - crates/mg-ui/src/settings.rs
  - crates/mg-ui/src/options.rs
  - crates/mg-render/src/gpu.rs
---

Decided: the window's adapter is chosen with egui-wgpu's native_adapter_selector, by mg_render::backend::pick_adapter over AdapterInfo (backend, vendor, device, kind). Automatic is wgpu's order, except an AMD (0x1002) GPU on Windows first taken through Vulkan is swapped for the Direct3D 12 adapter of the same device (or same kind). The instance keeps every backend so a chosen backend with no adapter can fall back. WGPU_BACKEND (Backends::from_env) decides over the setting and the AMD rule. Watch out: setting native_adapter_selector makes egui-wgpu skip request_adapter, so power preference ordering (wgpu-core's get_order by DeviceType, stable within backends) and the surface check (Adapter::is_surface_supported) are ours; re-check at a wgpu bump. The setting is read before run_native from eframe's app.ron (a RON map of key to RON text) with Settings::read_eframe_file; eframe's FileStorage can't be constructed from outside. Ruled out: #[serde(other)] on the GraphicsBackend enum (not allowed on externally tagged enums); a hand Deserialize maps unknown names to Automatic, read with deserialize_identifier (RON's bare identifier fails as a String).
