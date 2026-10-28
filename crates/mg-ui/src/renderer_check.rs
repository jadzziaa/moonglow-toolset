//! What Moonglow found of how its renderer draws on this machine: the
//! startup check's verdict (`apps/moonglow`, [`mg_render::backend::start`])
//! and the area view's watch ([`mg_render::watch`]); the notice under the
//! toolbar when either found it drawing nothing, with **Restart with …**
//! (the same GPU's other backend), and the restart itself.
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
        let (gpu, assets) = (vp.gpu.clone(), vp.textures_from(&game));
        // (No shader cache: the window's worker may be writing the same
        // programs to it, and its files are written apart by process, not
        // by thread. Compiled each run, 40 ms on the development machine.)
        let cache = None;
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned =
            std::thread::Builder::new().name("moonglow lit check".into()).spawn(move || {
                let _ = tx.send(mg_render::probe::probe_lit(&gpu, assets.as_ref(), cache));
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

/// What the lit check found, into the app: the debug log; a notice where
/// Moonglow's own shader draws nothing of a lit mesh (as the startup
/// check's, with Restart with …); and where the game's programs alone
/// draw nothing, Moonglow's own shader for the rest of the run.
pub fn lit_found(app: &mut Moonglow, lit: mg_render::probe::Lit) {
    crate::trace::note(format!("graphics: the lit check: {lit}"));
    let check = &mut app.renderer_check;
    let drawing = check.drawing();
    if let Err(why) = &lit.own {
        let said = format!(
            "Moonglow's check of lit drawing found that {drawing} draws nothing of a lit, \
             textured mesh ({why})."
        );
        app.log.warn(format!("Graphics: {said}"));
        check.trouble =
            Some(Trouble { said, offer: check.other, what: format!("the lit check: {why}") });
    } else if lit.game_fails() {
        let why = match &lit.game {
            Some(Err(why)) => why.clone(),
            _ => String::new(),
        };
        let said = format!(
            "The game's shaders draw nothing right through {drawing} ({why}): Moonglow draws \
             with its own shaders for the rest of this run (the Shaders button turns the game's \
             on again)."
        );
        app.log.warn(format!("Graphics: {said}"));
        check.game_shaders_off = Some(why.clone());
        check.trouble = Some(Trouble {
            said,
            offer: check.other,
            what: format!("the lit check, the game's shaders: {why}"),
        });
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
}

/// Moonglow to be started again once this run has ended
/// ([`Moonglow::relaunch`]): with the module that was open, through
/// `backend` (`environment`: as `WGPU_BACKEND`, which named this run's).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relaunch {
    pub module: Option<PathBuf>,
    pub backend: wgpu::Backend,
    pub environment: bool,
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
        self.trouble = Some(Trouble { said, offer, what: format!("the startup check: {why}") });
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
        said,
        offer: check.other,
        what: format!("the area view's watch: {failure}"),
    });
}

/// The notice of a renderer that draws nothing, under the toolbar while
/// there is one.
pub(crate) fn notice_bar(app: &mut Moonglow, ui: &mut egui::Ui) {
    let Some(trouble) = app.renderer_check.trouble.clone() else { return };
    let mut close = false;
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
                    } else {
                        "(Automatic passes over this backend on this driver from then on)".into()
                    };
                    if ui
                        .button(format!("Restart with {name}"))
                        .on_hover_text(format!(
                            "Closes Moonglow, asking about unsaved work first, and starts it \
                             again with the same module, drawing through {name} {after}"
                        ))
                        .clicked()
                    {
                        app.actions.push(Action::RestartGraphics(b));
                    }
                }
                None => {
                    ui.weak(
                        "No other backend is left to try on this GPU: see Help › User Manual › \
                         Troubleshooting.",
                    );
                }
            }
            if ui.small_button("×").on_hover_text("Close").clicked() {
                close = true;
            }
        });
    });
    if close {
        app.renderer_check.trouble = None;
    }
}

/// **Restart with `backend`**, once unsaved work is saved or discarded:
/// the failure recorded (Automatic) or the choice changed (Options), and
/// Moonglow quits, to be started again ([`Moonglow::relaunch`]).
pub(crate) fn restart(app: &mut Moonglow, backend: wgpu::Backend) {
    let check = app.renderer_check.clone();
    if !check.asked_for
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
    app.relaunch =
        Some(Relaunch { module: app.module_path(), backend, environment: check.from_environment });
}
