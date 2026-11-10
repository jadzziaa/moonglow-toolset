//! What Moonglow found of how its renderer draws on this machine: the
//! startup check's verdict (`apps/moonglow`, [`mg_render::backend::start`])
//! and the area view's watch ([`mg_render::watch`]); the notice under the
//! toolbar when either found it drawing nothing, with **Restart with …**
//! (the same GPU's other backend), and the restart itself. The notice
//! that the game's shaders alone draw wrong, which says the same each run
//! on that driver, offers **Don't Show Again** too.
//!
//! On Automatic, the restart records the backend's failure
//! ([`crate::settings::GraphicsFailure`]), which Automatic passes over
//! from then on (until the driver changes); a backend chosen in Options
//! is changed there instead (it is the builder's choice); one named by
//! `WGPU_BACKEND` is changed for the new run alone.

use std::path::PathBuf;

use crate::{Action, Moonglow};

/// The startup check, for the desktop app's adapter selector.
pub use mg_render::probe::{Probe, probe};

/// What the window draws with, and what was found wrong with it.
#[derive(Debug, Clone, Default)]
pub struct RendererCheck {
    /// The window's adapter.
    pub adapter: Option<wgpu::AdapterInfo>,
    /// The backend was asked for (Options › General, or `WGPU_BACKEND`):
    /// kept whatever it draws. `from_environment`: by `WGPU_BACKEND`.
    pub asked_for: bool,
    pub from_environment: bool,
    /// The same GPU's other backend, not recorded as failing: what the
    /// notice offers to restart with.
    pub other: Option<wgpu::Backend>,
    /// The backends Automatic passes over, as this run found: those it
    /// passed over as it started, and one it recorded then. Options says
    /// of the other records that they failed here and are drawn with all
    /// the same (no other backend is left).
    pub passes_over: Vec<wgpu::Backend>,
    /// The window's device was lost (why): nothing more is drawn. For the
    /// desktop app to tell the builder with a dialog of the system's, once.
    pub device_lost: Option<String>,
    /// What was found wrong, for the notice, until it is closed.
    pub trouble: Option<Trouble>,
    /// The watch has failed this run (it says so once).
    pub watch_failed: bool,
    /// The lit check found that the game's programs draw nothing (or
    /// wrong) where Moonglow's own draws right: why. Moonglow's own draws
    /// for the rest of the run, unless the Shaders switch is turned on
    /// again ([`Moonglow::game_shaders`]).
    pub game_shaders_off: Option<String>,
}

/// The lit check ([`mg_render::probe::probe_lit`]): run once a run, apart
/// from the window's thread, when the game's data is first read and the
/// 3D views' GPU is there; what it found.
#[derive(Debug, Default)]
pub struct LitCheck {
    started: bool,
    pending: Option<std::sync::mpsc::Receiver<mg_render::probe::Lit>>,
    pub found: Option<mg_render::probe::Lit>,
}

impl LitCheck {
    /// Whether it is under way, apart from the window's thread.
    pub(crate) fn checking(&self) -> bool {
        self.pending.is_some()
    }
}

/// Starts the lit check once the game's data and the 3D views' GPU are
/// there (in the desktop app: where jobs run in the background), and takes
/// what it found when it is done.
pub(crate) fn lit_check(app: &mut Moonglow) {
    if !app.lit_check.started {
        let (Some(vp), Some(game)) = (app.viewport.as_mut(), app.game.clone()) else { return };
        if !app.background_jobs {
            return;
        }
        app.lit_check.started = true;
        // (The game's programs read out of the game data first: holding it
        // while the check runs, a module opened meanwhile, as one given on
        // the command line is, could not layer its haks into it.)
        let read = std::time::Instant::now();
        let assets = GameShaders::of(&game.resman);
        crate::trace::note(format!(
            "graphics: the lit check: {} of the game's programs read in {:.0?}",
            assets.0.len(),
            read.elapsed()
        ));
        drop(game);
        let gpu = vp.gpu.clone();
        // (No shader cache: the window's worker may be writing the same
        // programs to it, and its files are written apart by process, not
        // by thread. Compiled each run, 40 ms on the development machine.)
        let cache = None;
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned =
            std::thread::Builder::new().name("moonglow lit check".into()).spawn(move || {
                let _ = tx.send(mg_render::probe::probe_lit(&gpu, &assets, cache));
            });
        if spawned.is_ok() {
            app.lit_check.pending = Some(rx);
        }
        return;
    }
    let Some(rx) = &app.lit_check.pending else { return };
    let Ok(lit) = rx.try_recv() else { return };
    app.lit_check.pending = None;
    lit_found(app, lit);
}

/// The game's programs (`.shd`), as the lit check reads them: out of the
/// game data, which it does not hold. Each with the game's own copy, which
/// the check draws with, and where the load order's comes from when it is
/// a builder's ([`mg_render::Assets::game_shader`], `shader_origin`).
struct GameShaders(std::collections::HashMap<mg_core::ResRef, Program>);

/// A program as the load order has it, the game's own copy, and where the
/// load order's comes from when it isn't the game's.
struct Program {
    loaded: String,
    own: Option<String>,
    origin: Option<String>,
}

impl GameShaders {
    fn of(resman: &mg_resman::ResMan) -> GameShaders {
        use mg_render::Assets;
        let read = |r: mg_core::ResRef| {
            let name = r.to_string();
            let loaded = resman.shader(&name)?;
            let (own, origin) = (resman.game_shader(&name), resman.shader_origin(&name));
            Some((r, Program { loaded, own, origin }))
        };
        GameShaders(resman.list(mg_core::ResType::SHD).into_iter().filter_map(read).collect())
    }

    fn program(&self, name: &str) -> Option<&Program> {
        self.0.get(&mg_core::ResRef::cut(name)?)
    }
}

impl mg_render::Assets for GameShaders {
    fn texture(&self, _: &str) -> Option<mg_render::LoadedTexture> {
        None
    }

    fn shader(&self, name: &str) -> Option<String> {
        self.program(name).map(|p| p.loaded.clone())
    }

    fn game_shader(&self, name: &str) -> Option<String> {
        self.program(name).and_then(|p| p.own.clone())
    }

    fn shader_origin(&self, name: &str) -> Option<String> {
        self.program(name).and_then(|p| p.origin.clone())
    }
}

/// What the lit check found, into the app: the debug log; a notice where
/// Moonglow's own shader draws a lit mesh wrong (as the startup check's,
/// with Restart with …); and where the game's programs alone draw it
/// wrong, Moonglow's own shader for the rest of the run. Each notice says
/// whether nothing of the mesh was drawn or it was drawn in the wrong
/// colors (where an area is drawn all the same).
pub fn lit_found(app: &mut Moonglow, lit: mg_render::probe::Lit) {
    crate::trace::note(format!("graphics: the lit check: {lit}"));
    let check = &mut app.renderer_check;
    let drawing = check.drawing();
    // (What was drawn of the mesh: nothing, or the mesh in the wrong
    // colors. An area may well be drawn where the colors are wrong.)
    let drew = |color, verb: &str| {
        if mg_render::probe::Lit::drew_nothing(color) {
            format!("{verb} nothing of the lit, textured test mesh")
        } else {
            format!("{verb} the lit, textured test mesh in the wrong colors")
        }
    };
    if let Err(why) = &lit.own {
        // (A panic compared no picture: the backend is not recorded for
        // it.)
        let said = if lit.panicked {
            format!("Moonglow's check of lit drawing failed through {drawing} ({why}).")
        } else {
            format!(
                "Moonglow's check of lit drawing found that {drawing} {} ({why}).",
                drew(lit.own_color, "draws")
            )
        };
        app.log.warn(format!("Graphics: {said}"));
        check.trouble = Some(Trouble {
            what: format!("the lit check: {why}"),
            record: !lit.panicked,
            ..Trouble::new(said, check.other)
        });
    } else if lit.game_fails() {
        let why = match &lit.game {
            Some(Err(why)) => why.clone(),
            _ => String::new(),
        };
        let found = if lit.panicked {
            "could not draw the lit, textured test mesh".to_string()
        } else {
            drew(lit.game_color, "draw")
        };
        let said = format!(
            "Moonglow's check of lit drawing found that the game's shaders {found} through \
             {drawing} ({why}), where Moonglow's own shaders draw it right: Moonglow draws with \
             its own shaders for the rest of this run (the Shaders button turns the game's on \
             again)."
        );
        check.game_shaders_off = Some(why.clone());
        // (Hidden on this GPU, driver and backend: the log alone says it,
        // as news, each run.)
        if check.adapter.as_ref().is_some_and(|a| app.settings.graphics_notice_hidden(a)) {
            app.log.info(format!(
                "Graphics: {said} (Its notice is hidden on this GPU and driver: Options › \
                 General shows it again.)"
            ));
        } else {
            app.log.warn(format!("Graphics: {said}"));
            check.trouble = Some(Trouble {
                what: format!("the lit check, the game's shaders: {why}"),
                hideable: check.adapter.is_some(),
                ..Trouble::new(said, check.other)
            });
        }
    } else if let Some(why) = &lit.fell_back {
        app.log.warn(format!(
            "Graphics: the game's lit shaders can't be made through {drawing} ({why}): Moonglow's \
             own shader draws what they would"
        ));
    }
    app.lit_check.found = Some(lit);
}

/// What the notice says, and the backend it offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trouble {
    pub said: String,
    pub offer: Option<wgpu::Backend>,
    /// What found it, as the record keeps it (`the startup check: …`).
    pub what: String,
    /// Offers Don't Show Again: the game's shaders alone draw wrong, where
    /// Moonglow's own draw for them ([`Settings::graphics_notices_hidden`]).
    ///
    /// [`Settings::graphics_notices_hidden`]: crate::Settings::graphics_notices_hidden
    pub hideable: bool,
    /// What was found is the backend's doing on this GPU and driver:
    /// Restart with … records it, on Automatic, which passes over it from
    /// then on. Off where the cause may as well be the content (tiles
    /// whose models couldn't be built, textures that never came) or no
    /// picture was compared: the restart is through the other backend for
    /// that run alone.
    pub record: bool,
}

impl Trouble {
    fn new(said: String, offer: Option<wgpu::Backend>) -> Trouble {
        Trouble { said, offer, what: String::new(), hideable: false, record: true }
    }
}

/// Moonglow to be started again once this run has ended
/// ([`Moonglow::relaunch`]): with the module that was open, through
/// `backend` (`environment`: as `WGPU_BACKEND`, which named this run's),
/// and (`start`, after an update) as the update says it is started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relaunch {
    pub module: Option<PathBuf>,
    pub backend: wgpu::Backend,
    pub environment: bool,
    pub start: Option<mg_update::Restart>,
}

impl RendererCheck {
    /// The adapter's backend and name (`Direct3D 12 on AMD Radeon RX 580`).
    fn drawing(&self) -> String {
        match &self.adapter {
            Some(a) => format!("{} on {}", mg_render::backend::backend_name(a.backend), a.name),
            None => "this GPU".into(),
        }
    }

    /// The startup check found the adapter drawing wrong (`why`) and it
    /// was kept: asked for, or no other backend drew right (`others`: the
    /// others checked, and what they drew wrong).
    pub fn startup_failed(&mut self, why: &str, others: &[(wgpu::Backend, String)]) {
        let mut said = format!(
            "Moonglow's startup check found that {} draws nothing of a scene ({why}).",
            self.drawing()
        );
        for (b, w) in others {
            said += &format!(" Nor does {} ({w}).", mg_render::backend::backend_name(*b));
        }
        let offer = self.other.filter(|_| self.asked_for && others.is_empty());
        self.trouble = Some(Trouble {
            what: format!("the startup check: {why}"),
            ..Trouble::new(said, offer)
        });
    }
}

/// The area view's watch failed (`failure`): said once a run, in the log
/// and the notice.
pub(crate) fn watch_failed(app: &mut Moonglow, failure: mg_render::watch::Failure) {
    let check = &mut app.renderer_check;
    if check.watch_failed {
        return;
    }
    check.watch_failed = true;
    let drawing = check.drawing();
    let fault = if failure.failed > 0 {
        "Tiles whose models couldn't be built (named in the log) or the graphics driver may be \
         at fault."
    } else if failure.waiting.is_some() {
        "Textures that are never read, or the graphics driver, may be at fault."
    } else {
        "The game's shaders (the Shaders button) or the graphics driver may be at fault."
    };
    let said =
        format!("The area view stopped drawing the area through {drawing}: {failure}. {fault}");
    app.log.warn(format!("Graphics: {said}"));
    crate::trace::note(format!("graphics watch: failed, {failure} ({drawing})"));
    let check = &mut app.renderer_check;
    check.trouble = Some(Trouble {
        what: format!("the area view's watch: {failure}"),
        // (Failed tiles or a wait for textures: the area's content may be
        // what is wrong, not the backend.)
        record: failure.failed == 0 && failure.waiting.is_none(),
        ..Trouble::new(said, check.other)
    });
}

/// The adapter Automatic took is one Moonglow draws wrong or slowly with
/// (OpenGL, a software adapter: nothing else was there), or such a one was
/// asked for: said in the log and the notice, with what to install.
pub fn fallback_taken(app: &mut Moonglow, fallback: mg_render::backend::Fallback) {
    let check = &mut app.renderer_check;
    let platform = mg_render::backend::Platform::current();
    let said = format!("Moonglow draws through {}, {}", check.drawing(), fallback.said(platform));
    app.log.warn(format!("Graphics: {said}"));
    // (Under a notice of what the startup check found, if there is one.)
    if check.trouble.is_none() {
        check.trouble = Some(Trouble {
            what: "a fallback adapter".into(),
            record: false,
            ..Trouble::new(said, None)
        });
    }
}

/// The window's device is lost (`why`): the unsaved work is kept in a
/// recovery copy at once, and the builder told (the log, and the desktop
/// app's dialog: the window draws nothing more).
pub(crate) fn device_lost(app: &mut Moonglow, why: String) {
    crate::trace::note(format!("graphics: the device was lost ({why})"));
    // (Unless Options › General keeps none, or nothing is unsaved.)
    let unsaved = !app.settings.no_autosave && app.has_unsaved_work();
    let kept = match unsaved.then(|| app.write_recovery()) {
        Some(Ok(copy)) => {
            format!(" Unsaved work is kept in a recovery copy ({}).", copy.display())
        }
        _ => String::new(),
    };
    let said = format!(
        "The graphics device was lost ({why}): the graphics driver was reset or updated, or the \
         GPU removed. Moonglow can draw nothing more in this run.{kept} Close Moonglow and start \
         it again."
    );
    app.log.error(said.clone());
    app.renderer_check.device_lost = Some(said);
}

/// The notice of a renderer that draws nothing, under the toolbar while
/// there is one.
pub(crate) fn notice_bar(app: &mut Moonglow, ui: &mut egui::Ui) {
    let Some(trouble) = app.renderer_check.trouble.clone() else { return };
    let (mut close, mut hide) = (false, false);
    egui::Panel::top("renderer-notice").show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("⚠").color(ui.visuals().warn_fg_color));
            ui.label(&trouble.said);
            match trouble.offer {
                Some(b) => {
                    let name = mg_render::backend::backend_name(b);
                    let check = &app.renderer_check;
                    let after = if check.from_environment {
                        "(for that run: WGPU_BACKEND names this one's)".to_string()
                    } else if check.asked_for {
                        format!("(Options › General › Graphics backend becomes {name})")
                    } else if !trouble.record {
                        "(for that run alone: what is wrong may not be this backend)".to_string()
                    } else {
                        "(Automatic passes over this backend on this driver from then on)".into()
                    };
                    if ui
                        .button(format!("Restart with {name}"))
                        .on_hover_text(format!(
                            "Restart Moonglow with the same module, drawing through {name} {after}"
                        ))
                        .clicked()
                    {
                        app.actions.push(Action::RestartGraphics(b));
                    }
                }
                None if trouble.what == "a fallback adapter" => {
                    ui.weak("See Help › User Manual › Troubleshooting.");
                }
                None => {
                    ui.weak(
                        "No other backend is left to try on this GPU: see Help › User Manual › \
                         Troubleshooting.",
                    );
                }
            }
            if trouble.hideable
                && ui
                    .button("Don't Show Again")
                    .on_hover_text(
                        "Hide this notice on this GPU, driver and backend. Options › General shows \
                         it again.",
                    )
                    .clicked()
            {
                hide = true;
            }
            if ui.small_button("×").on_hover_text("Close").clicked() {
                close = true;
            }
        });
    });
    if hide && let Some(info) = &app.renderer_check.adapter {
        let hidden = crate::settings::GraphicsFailure::of(info, &trouble.what);
        crate::trace::note(format!("graphics: the lit check's notice hidden ({})", hidden.driver));
        app.settings.hide_graphics_notice(hidden);
    }
    if close || hide {
        app.renderer_check.trouble = None;
    }
}

/// **Restart with `backend`**, once unsaved work is saved or discarded:
/// the failure recorded (Automatic) or the choice changed (Options), and
/// Moonglow quits, to be started again ([`Moonglow::relaunch`]).
pub(crate) fn restart(app: &mut Moonglow, backend: wgpu::Backend) {
    let check = app.renderer_check.clone();
    // (Recorded only where the backend is what is wrong; else the other
    // backend for the new run alone, as `WGPU_BACKEND` names one.)
    let record = check.trouble.as_ref().is_none_or(|t| t.record);
    if !check.asked_for
        && record
        && let Some(info) = &check.adapter
    {
        let what = check.trouble.as_ref().map_or("the area view's watch", |t| t.what.as_str());
        app.settings.record_graphics_failure(crate::settings::GraphicsFailure::of(info, what));
    } else if check.asked_for && !check.from_environment {
        app.settings.graphics_backend = crate::settings::GraphicsBackend::of(backend);
    }
    crate::trace::note(format!(
        "graphics: restart with {}",
        mg_render::backend::backend_name(backend)
    ));
    app.relaunch = Some(Relaunch {
        module: app.module_path(),
        backend,
        environment: check.from_environment || (!check.asked_for && !record),
        start: None,
    });
}

#[cfg(test)]
mod tests {
    use mg_render::Assets;
    use mg_resman::{LayerClass, MemContainer, ResKey, ResMan, priority};

    use super::GameShaders;

    /// The lit check's programs, read apart from the game data, keep the
    /// game's own copy it draws with and where a builder's comes from (the
    /// two fixes of one day met here: read up front, it had drawn the
    /// builder's `inc_lighting` again).
    #[test]
    fn the_lit_check_s_programs_keep_the_game_s_own_copy() {
        let key = |name: &str| ResKey::new(name.parse().unwrap(), mg_core::ResType::SHD);
        let (mut game, mut over) = (MemContainer::new(), MemContainer::new());
        game.insert(key("inc_lighting"), b"game".to_vec());
        game.insert(key("fslit"), b"stock".to_vec());
        over.insert(key("inc_lighting"), b"builder".to_vec());
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "key:nwn_base", LayerClass::Key, game);
        rm.add(priority::OVERRIDE, "override", LayerClass::Directory, over);
        let shaders = GameShaders::of(&rm);
        assert_eq!(shaders.shader("inc_lighting").as_deref(), Some("builder"));
        assert_eq!(shaders.game_shader("inc_lighting").as_deref(), Some("game"));
        assert_eq!(shaders.shader_origin("inc_lighting").as_deref(), Some("override"));
        assert_eq!(shaders.game_shader("fslit").as_deref(), Some("stock"));
        assert_eq!(shaders.shader_origin("fslit"), None);
        assert_eq!(shaders.game_shader("nothing"), None);
    }

    /// Restart with …, on Automatic: a watch that found the GPU drawing
    /// nothing records the backend, which Automatic passes over from then
    /// on; one that failed with tiles left out or textures waited for (the
    /// area's content may be what is wrong) records nothing, and the new
    /// run is through the other backend for itself alone.
    #[test]
    fn a_watch_failure_of_the_content_s_records_no_backend() {
        let mut info = wgpu::AdapterInfo::new(wgpu::DeviceType::DiscreteGpu, wgpu::Backend::Dx12);
        (info.vendor, info.device, info.name) = (0x1002, 0x67df, "Radeon RX 580".into());
        let failure = |failed, waiting| mg_render::watch::Failure {
            expected: 100,
            drawn: 3,
            seconds: 5.0,
            failed,
            waiting,
        };
        for (failure, recorded) in
            [(failure(0, None), true), (failure(40, None), false), (failure(0, Some(20.0)), false)]
        {
            let mut app = crate::Moonglow::new(None, Box::new(crate::NoDialogs::default()));
            app.renderer_check.adapter = Some(info.clone());
            app.renderer_check.other = Some(wgpu::Backend::Vulkan);
            super::watch_failed(&mut app, failure);
            let trouble = app.renderer_check.trouble.clone().expect("the notice");
            assert_eq!((trouble.record, trouble.offer), (recorded, Some(wgpu::Backend::Vulkan)));
            super::restart(&mut app, wgpu::Backend::Vulkan);
            assert_eq!(app.settings.graphics_failures.len(), usize::from(recorded), "{failure}");
            let again = app.relaunch.clone().expect("the restart");
            assert_eq!((again.backend, again.environment), (wgpu::Backend::Vulkan, !recorded));
        }
    }

    /// Options tells the backends Automatic passes over from those it
    /// draws with all the same (one backend on the platform: the record
    /// was said to be passed over, and Try Again offered for nothing).
    #[test]
    fn options_says_which_recorded_backends_are_passed_over() {
        let mut info = wgpu::AdapterInfo::new(wgpu::DeviceType::DiscreteGpu, wgpu::Backend::Dx12);
        (info.vendor, info.device, info.name) = (0x1002, 0x67df, "Radeon RX 580".into());
        info.driver_info = "25.9.1".into();
        let dx12 = crate::settings::GraphicsFailure::of(&info, "the startup check: nothing drawn");
        info.backend = wgpu::Backend::Vulkan;
        let vulkan = crate::settings::GraphicsFailure::of(&info, "the area view's watch");
        let said = |passes: &[wgpu::Backend]| {
            crate::options::graphics_failures_said(&[dx12.clone(), vulkan.clone()], passes)
        };
        let both = said(&[wgpu::Backend::Dx12]);
        assert!(
            both.starts_with(
                "Automatic passes over Direct3D 12 on Radeon RX 580 (driver 25.9.1): it drew \
                 wrong here (the startup check: nothing drawn, less than a minute ago). Vulkan \
                 on Radeon RX 580 (driver 25.9.1) drew wrong here (the area view's watch, "
            ),
            "{both}"
        );
        assert!(both.ends_with("where no other backend is left."), "{both}");
        assert!(said(&[]).starts_with("Direct3D 12 on Radeon RX 580 (driver 25.9.1), Vulkan on"));
        assert!(!said(&[]).contains("passes over"));
    }
}
