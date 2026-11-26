//! The viewers a resource is seen with wherever it is shown (M5's W6, the
//! owner's ruling 49, "share the viewers"): the hak editor's Preview and
//! the read-only resource tab draw a picture (a TGA, a DDS, a PLT in each
//! layer's first color) and play a sound (a WAV, a BMU) by this one code.
//! A picture is decoded off the frame, once, and kept as long as what
//! shows it keeps it; a sound plays on the Preview channel, one at a time
//! (another Play anywhere takes the channel), and stops where the place
//! that plays it is drawn no more (closed, behind another tab, folded).

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use mg_core::ResType;
use mg_resman::ResKey;

use crate::Moonglow;
use crate::audio::Channel;
use egui::Ui;

/// Whether a resource of type `t` is shown as a picture.
pub(crate) fn is_picture(t: ResType) -> bool {
    matches!(t, ResType::TGA | ResType::DDS | ResType::PLT)
}

/// Whether a resource of type `t` is played as a sound.
pub(crate) fn is_sound(t: ResType) -> bool {
    matches!(t, ResType::WAV | ResType::BMU)
}

/// Pictures decoded, all told.
static DECODED: AtomicUsize = AtomicUsize::new(0);
/// Enabled when tests query [`decoded_of`].
static TRACK_DECODED_OF: AtomicBool = AtomicBool::new(false);
/// Each picture's decodes, by its resource (the tests' count: decoded
/// once, not per frame).
static DECODED_OF: Mutex<Option<std::collections::HashMap<ResKey, usize>>> = Mutex::new(None);
static HELD_COUNT: AtomicUsize = AtomicUsize::new(0);
/// Pictures whose decodes wait until they are let go of (the tests': the
/// frames go on meanwhile).
static HELD: Mutex<Vec<ResKey>> = Mutex::new(Vec::new());
static FAILS_COUNT: AtomicUsize = AtomicUsize::new(0);
/// Pictures whose decodes panic (the tests': nothing is left waiting, the
/// window decodes on).
static FAILS: Mutex<Vec<ResKey>> = Mutex::new(Vec::new());

/// How many pictures a window decodes at once; the rest wait their turn
/// (a session restoring many picture tabs starts no more than this).
const DECODERS: usize = 4;

/// A window's picture decodes: a few threads at a time taking them in
/// turn, and how many are waiting or under way (the window's own work,
/// [`Moonglow::under_way`]; a decode is done when its picture is stored
/// for the frame, drawn or not).
#[derive(Default)]
pub(crate) struct Decodes {
    shared: Arc<DecodesShared>,
}

#[derive(Default)]
struct DecodesShared {
    queue: Mutex<Queue>,
    /// Decodes waiting or under way. Taken down with `Release` after the
    /// decode's picture is stored and `stored` set, and read with
    /// `Acquire` (`busy`), so a frame that sees it at 0 sees `stored` too
    /// and goes on one more frame to make the texture; relaxed, the two
    /// could be seen in either order and `settle` stop a frame early.
    pending: AtomicUsize,
    /// A decode stored since the frame began: one more frame makes its
    /// texture where it is drawn (cleared as each frame begins, drawn or
    /// not, so a hidden picture holds nothing).
    stored: std::sync::atomic::AtomicBool,
}

#[derive(Default)]
struct Queue {
    /// Each with its picture's mark (`Pending::id`), taken from the front.
    jobs: std::collections::VecDeque<(usize, Box<dyn FnOnce() + Send>)>,
    workers: usize,
}

impl std::fmt::Debug for Decodes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Decodes({})", self.pending())
    }
}

impl Decodes {
    /// How many pictures wait to be decoded or are being decoded.
    pub(crate) fn pending(&self) -> usize {
        self.shared.pending.load(Ordering::Acquire)
    }

    /// Whether the window has decodes to wait for: waiting, under way, or
    /// stored since this frame began (its texture made in the next). The
    /// count is read first (`Acquire`): see `DecodesShared::pending`.
    pub(crate) fn busy(&self) -> bool {
        self.pending() > 0 || self.shared.stored.load(Ordering::Relaxed)
    }

    /// How many decodes wait for a thread to take them.
    pub(crate) fn untaken(&self) -> usize {
        self.shared.queue.lock().unwrap_or_else(|e| e.into_inner()).jobs.len()
    }

    /// How many threads are decoding.
    pub(crate) fn workers(&self) -> usize {
        self.shared.queue.lock().unwrap_or_else(|e| e.into_inner()).workers
    }

    /// `job` (for the picture marked `id`) run on one of the window's
    /// decoding threads, in its turn.
    fn push(&self, id: usize, job: Box<dyn FnOnce() + Send>) {
        let mut q = self.shared.queue.lock().unwrap_or_else(|e| e.into_inner());
        q.jobs.push_back((id, job));
        staff(&self.shared, &mut q);
    }
}

impl DecodesShared {
    /// The picture marked `id`, drawn while it waits its turn, decoded
    /// next: the one in sight before hidden ones (a session's restored
    /// tabs behind it).
    fn hurry(&self, id: usize) {
        let mut q = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(at) = q.jobs.iter().position(|(i, _)| *i == id).filter(|&at| at > 0) {
            let job = q.jobs.remove(at).expect("just found");
            q.jobs.push_front(job);
        }
    }
}

/// A decoding thread started where jobs wait and fewer than
/// [`DECODERS`] are at work (`q`: the queue, locked).
fn staff(shared: &Arc<DecodesShared>, q: &mut Queue) {
    if q.jobs.is_empty() || q.workers >= DECODERS {
        return;
    }
    let held = shared.clone();
    let spawned = std::thread::Builder::new().name("picture".into()).spawn(move || {
        let worker = Worker(held);
        work(&worker.0);
    });
    match spawned {
        Ok(_) => q.workers += 1,
        // (No thread to be had and none at work: the jobs dropped, each
        // picture saying it could not be read.)
        Err(_) if q.workers == 0 => q.jobs.clear(),
        Err(_) => {}
    }
}

/// A decoding thread's place in the count, given up however the thread
/// ends (no job left, or a decode that panicked and took the thread): a
/// thread that died uncounted would leave the window short of decoders,
/// and after four every later picture would wait for ever. Jobs still
/// waiting get a thread in its place.
struct Worker(Arc<DecodesShared>);

impl Drop for Worker {
    fn drop(&mut self) {
        let mut q = self.0.queue.lock().unwrap_or_else(|e| e.into_inner());
        q.workers -= 1;
        staff(&self.0, &mut q);
    }
}

/// A decoding thread: the window's queued decodes taken in turn until
/// none waits. A decode that panics ends the thread (its `Done` fails
/// its picture, its `Worker` gives up its place).
fn work(shared: &DecodesShared) {
    loop {
        let job = {
            let mut q = shared.queue.lock().unwrap_or_else(|e| e.into_inner());
            match q.jobs.pop_front() {
                Some((_, job)) => job,
                None => return,
            }
        };
        job();
    }
}

/// A queued decode's end, however it ends (decoded, panicked, dropped
/// unrun): its picture failed if nothing was stored, the window's count
/// down, a frame asked for.
struct Done {
    slot: std::sync::Weak<Mutex<Option<Decoded>>>,
    shared: Arc<DecodesShared>,
    ctx: egui::Context,
}

impl Drop for Done {
    fn drop(&mut self) {
        if let Some(slot) = self.slot.upgrade() {
            let mut slot = slot.lock().unwrap_or_else(|e| e.into_inner());
            if slot.is_none() {
                *slot = Some(Err("The picture could not be read.".into()));
            }
        }
        self.shared.stored.store(true, Ordering::Relaxed);
        self.shared.pending.fetch_sub(1, Ordering::Release);
        self.ctx.request_repaint();
    }
}

/// The read-only header's More's check row that shows a picture's or a
/// sound's first bytes in place of it.
pub const SHOW_BYTES: &str = "Show Bytes";
/// The same for a model: its text in place of its drawing.
pub const SHOW_TEXT: &str = "Show as Text";

/// How many pictures the viewers have decoded since the program began.
pub fn decoded() -> usize {
    DECODED.load(Ordering::Relaxed)
}

/// How many times the picture resource `key` has been decoded.
#[doc(hidden)]
pub fn decoded_of(key: ResKey) -> usize {
    TRACK_DECODED_OF.store(true, Ordering::Relaxed);
    let kept = DECODED_OF.lock().unwrap_or_else(|e| e.into_inner());
    kept.as_ref().and_then(|m| m.get(&key).copied()).unwrap_or(0)
}

/// Holds the decodes of `keys`' pictures until [`release`]d, for a test
/// to see the frames go on meanwhile (each test its own keys: tests run
/// side by side).
#[doc(hidden)]
pub fn hold(keys: &[ResKey]) {
    let mut held = HELD.lock().unwrap_or_else(|e| e.into_inner());
    held.extend_from_slice(keys);
    HELD_COUNT.store(held.len(), Ordering::Relaxed);
}

/// Lets the decodes of `keys`' pictures go on.
#[doc(hidden)]
pub fn release(keys: &[ResKey]) {
    let mut held = HELD.lock().unwrap_or_else(|e| e.into_inner());
    held.retain(|k| !keys.contains(k));
    HELD_COUNT.store(held.len(), Ordering::Relaxed);
}

/// Makes the decode of `key`'s picture panic (`None`: none), for a test
/// to see that nothing is left waiting.
#[doc(hidden)]
pub fn fail(keys: &[ResKey]) {
    let mut fails = FAILS.lock().unwrap_or_else(|e| e.into_inner());
    fails.extend_from_slice(keys);
    FAILS_COUNT.store(fails.len(), Ordering::Relaxed);
}

/// Lets the decodes of `keys`' pictures succeed again.
#[doc(hidden)]
pub fn heal(keys: &[ResKey]) {
    let mut fails = FAILS.lock().unwrap_or_else(|e| e.into_inner());
    fails.retain(|k| !keys.contains(k));
    FAILS_COUNT.store(fails.len(), Ordering::Relaxed);
}

type Decoded = Result<(egui::ColorImage, [usize; 2]), String>;

/// A picture to show: being decoded on a thread of its own, then a
/// texture made once on the frame, kept with it.
pub(crate) struct Picture {
    stage: Mutex<Stage>,
    /// How it is zoomed and where (a new picture is a fresh one, fitted).
    view: Mutex<View>,
}

/// A picture's zoom: `None` is fitted (and follows its room), else the
/// screen points to a pixel of the picture; and where its center is,
/// from the room's.
#[derive(Debug, Default, Clone, Copy)]
struct View {
    scale: Option<f32>,
    offset: egui::Vec2,
}

/// The most a picture is magnified.
const MAX_ZOOM: f32 = 16.0;
/// What a wheel notch (40 points) and a key press multiply the zoom by.
const ZOOM_STEP: f32 = 1.25;

/// Where a zoomed picture of `size` at `scale` goes in `area`: centered
/// where it is smaller than `area`, else kept over all of `area`.
fn placed(area: egui::Rect, size: egui::Vec2, scale: f32, offset: egui::Vec2) -> egui::Rect {
    let shown = size * scale;
    let slack = ((shown - area.size()) / 2.0).max(egui::Vec2::ZERO);
    let offset = offset.clamp(-slack, slack);
    egui::Rect::from_center_size(area.center() + offset, shown)
}

/// What the picture of `key` was last drawn at, for the tests: where it
/// is and its scale.
#[doc(hidden)]
pub fn picture_drawn(ctx: &egui::Context, key: ResKey) -> Option<(egui::Rect, f32)> {
    ctx.data(|d| d.get_temp(egui::Id::new(("viewer-picture-drawn", key))))
}

impl std::fmt::Debug for Picture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Picture")
    }
}

enum Stage {
    Decoding(Pending),
    /// Its texture and its size in pixels (the texture may be smaller,
    /// where the picture is larger than the GPU takes), the image it was
    /// made from and whether its texels are sampled sharp (above 1:1; the
    /// texture is made again where that changes).
    Ready(egui::TextureHandle, [usize; 2], Arc<egui::ColorImage>, bool),
    Failed(String),
}

/// A decode under way: where its thread puts what it made, and the
/// texture's name.
struct Pending {
    slot: Arc<Mutex<Option<Decoded>>>,
    name: String,
    /// Its mark in the window's queue, and the queue (drawn while it
    /// waits, it goes first: `DecodesShared::hurry`).
    id: usize,
    shared: Arc<DecodesShared>,
}

impl Picture {
    /// `data`, a picture resource `key`, decoded on a thread of its own (a
    /// PLT in each layer's first color, from the game's palettes, which are
    /// read here, once for the window). Shown as it comes.
    pub(crate) fn decode(
        app: &mut Moonglow,
        ctx: &egui::Context,
        key: ResKey,
        data: Arc<[u8]>,
    ) -> Picture {
        let palettes: Vec<Option<Arc<mg_image::Rgba>>> = match (key.restype, app.loader()) {
            (ResType::PLT, Some(l)) => {
                mg_image::plt::PALETTES.iter().map(|p| l.palette(p)).collect()
            }
            _ => vec![None; 10],
        };
        let side = ctx.input(|i| i.max_texture_side).max(16);
        let slot = Arc::new(Mutex::new(None));
        let shared = app.decodes.shared.clone();
        let id = Arc::as_ptr(&slot) as usize;
        let name = format!("viewer-{key}");
        let pending = Pending { slot: slot.clone(), name, id, shared: shared.clone() };
        shared.pending.fetch_add(1, Ordering::Relaxed);
        let done = Done { slot: Arc::downgrade(&slot), shared, ctx: ctx.clone() };
        app.decodes.push(
            id,
            Box::new(move || {
                // (Its picture let go of before its turn came: nothing to do.)
                if done.slot.strong_count() == 0 {
                    return;
                }
                while HELD_COUNT.load(Ordering::Relaxed) > 0
                    && HELD.lock().unwrap_or_else(|e| e.into_inner()).contains(&key)
                {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                if FAILS_COUNT.load(Ordering::Relaxed) > 0
                    && FAILS.lock().unwrap_or_else(|e| e.into_inner()).contains(&key)
                {
                    panic!("a picture's decode failed on purpose (the tests')");
                }
                let made = decode_now(key.restype, &data, &palettes, side);
                DECODED.fetch_add(1, Ordering::Relaxed);
                if TRACK_DECODED_OF.load(Ordering::Relaxed) {
                    *DECODED_OF
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .get_or_insert_default()
                        .entry(key)
                        .or_default() += 1;
                }
                if let Some(slot) = done.slot.upgrade() {
                    *slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(made);
                }
                drop(done);
            }),
        );
        Picture { stage: Mutex::new(Stage::Decoding(pending)), view: Mutex::default() }
    }

    /// Its texture and size once decoded (the texture made here, once),
    /// why it can't be shown, or `Ok(None)` while it is decoded.
    fn texture(
        &self,
        ctx: &egui::Context,
    ) -> Result<Option<(egui::TextureHandle, [usize; 2])>, String> {
        let mut stage = self.stage.lock().unwrap_or_else(|e| e.into_inner());
        if let Stage::Decoding(p) = &*stage {
            let made = p.slot.lock().unwrap_or_else(|e| e.into_inner()).take();
            match made {
                None => {
                    p.shared.hurry(p.id);
                    return Ok(None);
                }
                Some(Ok((image, size))) => {
                    let image = Arc::new(image);
                    let texture =
                        ctx.load_texture(&p.name, image.clone(), egui::TextureOptions::LINEAR);
                    *stage = Stage::Ready(texture, size, image, false);
                }
                Some(Err(e)) => *stage = Stage::Failed(e),
            }
        }
        match &*stage {
            Stage::Ready(t, size, ..) => Ok(Some((t.clone(), *size))),
            Stage::Failed(e) => Err(e.clone()),
            Stage::Decoding(_) => Ok(None),
        }
    }

    /// Its texels sampled sharp (a magnified picture's pixels stay
    /// crisp) or smoothed, the texture made again where that changes.
    fn sharpen(&self, sharp: bool) {
        let mut stage = self.stage.lock().unwrap_or_else(|e| e.into_inner());
        if let Stage::Ready(t, _, image, now) = &mut *stage
            && *now != sharp
        {
            let options =
                if sharp { egui::TextureOptions::NEAREST } else { egui::TextureOptions::LINEAR };
            t.set(image.clone(), options);
            *now = sharp;
        }
    }

    /// Draws it in `room` (the hak editor's Preview's look, the Hak
    /// board's): fitted at first, never enlarged, its size under it; the
    /// wheel zooms about the pointer (up to 16 times), a drag pans a
    /// larger picture, a double click goes between fitted and actual
    /// pixels, `+`, `-` and `0` over it zoom and fit, and while it is
    /// zoomed its percent is said at the corner. While it is decoded,
    /// that it is; why it can't be shown, where it can't.
    pub(crate) fn ui(&self, ui: &mut Ui, key: ResKey, room: egui::Rect) {
        match self.texture(ui.ctx()) {
            Ok(Some((texture, size))) => {
                let caption = format!("{} by {} pixels", size[0], size[1]);
                let size = egui::vec2(size[0] as f32, size[1] as f32);
                let area = room.shrink2(egui::vec2(12.0, 12.0)).with_max_y(room.bottom() - 30.0);
                let fit = (area.width() / size.x.max(1.0))
                    .min(area.height() / size.y.max(1.0))
                    .clamp(0.01, 1.0);
                let mut view = self.view.lock().unwrap_or_else(|e| e.into_inner());
                let scale = view.scale.unwrap_or(fit).clamp(fit, MAX_ZOOM);
                let shown = placed(area, size, scale, view.offset);
                // (The room to pan in and zoom over, all of it.)
                let r = ui.interact(
                    area,
                    ui.id().with(("viewer-picture-room", key)),
                    egui::Sense::click_and_drag(),
                );
                let mut next = (scale, shown);
                let zoom_to = |to: f32, at: egui::Pos2, next: &mut (f32, egui::Rect)| {
                    // The pixel under `at` stays under it.
                    let to = to.clamp(fit, MAX_ZOOM);
                    let c = at + (next.1.center() - at) * (to / next.0);
                    next.0 = to;
                    next.1 = placed(area, size, to, c - area.center());
                };
                if r.hovered() || r.dragged() {
                    let at = ui.input(|i| i.pointer.hover_pos()).unwrap_or(area.center());
                    let wheel = ui.input(|i| i.smooth_scroll_delta.y);
                    if wheel != 0.0 {
                        zoom_to(next.0 * ZOOM_STEP.powf(wheel / 40.0), at, &mut next);
                        // (Not the panel's around it too.)
                        ui.input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
                    }
                    if !ui.ctx().egui_wants_keyboard_input() {
                        let key_down = |ui: &Ui, k: egui::Key, shift: bool| {
                            let m =
                                if shift { egui::Modifiers::SHIFT } else { egui::Modifiers::NONE };
                            ui.input_mut(|i| i.consume_key(m, k))
                        };
                        let centre = area.center();
                        if [egui::Key::Plus, egui::Key::Equals]
                            .into_iter()
                            .any(|k| key_down(ui, k, false) || key_down(ui, k, true))
                        {
                            zoom_to(next.0 * ZOOM_STEP, centre, &mut next);
                        }
                        if key_down(ui, egui::Key::Minus, false) {
                            zoom_to(next.0 / ZOOM_STEP, centre, &mut next);
                        }
                        if key_down(ui, egui::Key::Num0, false) {
                            zoom_to(fit, centre, &mut next);
                        }
                    }
                }
                let pannable =
                    next.1.width() > area.width() + 0.5 || next.1.height() > area.height() + 0.5;
                if r.dragged_by(egui::PointerButton::Primary) && pannable {
                    let moved = r.drag_delta();
                    next.1 = placed(area, size, next.0, next.1.center() - area.center() + moved);
                }
                if r.double_clicked()
                    && let Some(at) = r.interact_pointer_pos()
                {
                    if (next.0 - fit).abs() > 1e-4 {
                        zoom_to(fit, at, &mut next);
                    } else {
                        // Actual pixels (a small picture, fitted at 1:1 already,
                        // 4:1) with the point double clicked in the middle.
                        let to = if fit >= 0.999 { 4.0 } else { 1.0 };
                        let pixel = (at - next.1.min) / next.0;
                        next.0 = to;
                        next.1 = placed(area, size, to, (size * 0.5 - pixel) * to);
                    }
                }
                let (scale, shown) = next;
                // Fitted again where it is back at the fit.
                let fitted = (scale - fit).abs() <= 1e-4;
                *view = if fitted {
                    View::default()
                } else {
                    View { scale: Some(scale), offset: shown.center() - area.center() }
                };
                drop(view);
                self.sharpen(scale > 1.0);
                let visible = shown.intersect(area);
                let uv = egui::Rect::from_min_max(
                    egui::pos2(
                        (visible.min.x - shown.min.x) / shown.width(),
                        (visible.min.y - shown.min.y) / shown.height(),
                    ),
                    egui::pos2(
                        (visible.max.x - shown.min.x) / shown.width(),
                        (visible.max.y - shown.min.y) / shown.height(),
                    ),
                );
                ui.painter().image(texture.id(), visible, uv, egui::Color32::WHITE);
                ui.ctx().data_mut(|d| {
                    d.insert_temp(egui::Id::new(("viewer-picture-drawn", key)), (shown, scale));
                });
                let r = r.on_hover_text(
                    "Scroll to zoom, drag to pan, double click to go between fitted and actual size",
                );
                if pannable && r.hovered() {
                    ui.ctx().set_cursor_icon(if r.dragged() {
                        egui::CursorIcon::Grabbing
                    } else {
                        egui::CursorIcon::Grab
                    });
                }
                let image = ui.interact(
                    visible,
                    ui.id().with(("viewer-picture", key)),
                    egui::Sense::hover(),
                );
                image.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Image,
                        true,
                        format!("{key}, {caption}"),
                    )
                });
                let _ = caption_at(ui, room, &caption);
                if !fitted {
                    percent_at(ui, key, area, scale * 100.0);
                }
            }
            Ok(None) => {
                const READING: &str = "Reading the picture…";
                let rect = caption_at(ui, room, READING);
                let r =
                    ui.interact(rect, ui.id().with(("viewer-reading", key)), egui::Sense::hover());
                r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, READING));
            }
            Err(e) => {
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(room.shrink(10.0)));
                child.colored_label(child.visuals().error_fg_color, e);
            }
        }
    }
}

/// A picture resource's pixels as a texture takes them, top row first,
/// no larger than `side` across (a larger one taken every so many pixels).
fn decode_now(
    t: ResType,
    data: &[u8],
    palettes: &[Option<Arc<mg_image::Rgba>>],
    side: usize,
) -> Decoded {
    let rgba = match t {
        ResType::PLT => {
            let plt = mg_image::plt::Plt::read(data).map_err(|e| e.to_string())?;
            let refs: [Option<&mg_image::Rgba>; 10] =
                std::array::from_fn(|i| palettes.get(i).and_then(|p| p.as_deref()));
            plt.colorize(&refs, [0; 10])
        }
        _ => mg_image::read(t, data).map_err(|e| e.to_string())?.to_rgba(),
    };
    let rgba = rgba.top_down();
    let (w, h) = (rgba.width as usize, rgba.height as usize);
    if rgba.data.len() != w * h * 4 || w == 0 || h == 0 {
        return Err("the picture has no pixels".to_string());
    }
    let step = w.max(h).div_ceil(side).max(1);
    if step == 1 {
        return Ok((egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba.data), [w, h]));
    }
    let (sw, sh) = (w.div_ceil(step), h.div_ceil(step));
    let mut small = Vec::with_capacity(sw * sh * 4);
    for y in 0..sh {
        for x in 0..sw {
            let from = ((y * step) * w + x * step) * 4;
            small.extend_from_slice(&rgba.data[from..from + 4]);
        }
    }
    Ok((egui::ColorImage::from_rgba_unmultiplied([sw, sh], &small), [w, h]))
}

/// The zoom `percent` of the picture `key`, quiet, at the top right of
/// `room`.
fn percent_at(ui: &mut Ui, key: ResKey, room: egui::Rect, percent: f32) {
    let t = mg_uikit::tokens(ui.ctx());
    let words = format!("{}%", percent.round());
    let galley =
        ui.painter().layout_no_wrap(words.clone(), egui::FontId::proportional(11.0), t.text_3);
    let rect = egui::Rect::from_min_size(
        egui::pos2(room.right() - galley.size().x - 14.0, room.top() + 4.0),
        galley.size() + egui::vec2(10.0, 4.0),
    );
    ui.painter().rect_filled(rect, 3.0, t.raised.gamma_multiply(0.85));
    ui.painter().galley(rect.min + egui::vec2(5.0, 2.0), galley, t.text_3);
    let r = ui.interact(rect, ui.id().with(("viewer-picture-zoom", key)), egui::Sense::hover());
    r.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Label, true, format!("Zoom {words}"))
    });
}

/// A quiet caption at the foot of `room`, centered. Where it stands.
pub(crate) fn caption_at(ui: &Ui, room: egui::Rect, words: &str) -> egui::Rect {
    let t = mg_uikit::tokens(ui.ctx());
    let font = egui::FontId::proportional(11.0);
    let galley = ui.painter().layout(words.to_string(), font, t.text_3, room.width() - 20.0);
    let at =
        egui::pos2(room.center().x - galley.size().x / 2.0, room.bottom() - 10.0 - galley.size().y);
    let rect = egui::Rect::from_min_size(at, galley.size());
    ui.painter().galley(at, galley, t.text_3);
    rect
}

/// Who plays a sound on the Preview channel through a viewer: a hak's
/// Preview (its id), or a read-only tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Player {
    Hak(u32),
    Tab(crate::Tab),
}

/// The sound a viewer plays: who, which, and the pass it was last drawn
/// in (where it is drawn no more, it stops: [`follow`]).
#[derive(Debug, Clone)]
pub(crate) struct Playing {
    pub(crate) by: Player,
    pub(crate) key: ResKey,
    pass: u64,
}

/// Whether `by` plays `key` now (another Play anywhere took the channel,
/// or it ended: not).
pub(crate) fn plays(app: &Moonglow, by: &Player, key: ResKey) -> bool {
    app.viewer_sound.as_ref().is_some_and(|p| p.by == *by && p.key == key)
        && app.speaker.playing(Channel::Preview)
}

/// The sound `by` plays stopped, where it is still `by`'s.
pub(crate) fn stop_by(app: &mut Moonglow, by: &Player) {
    if app.viewer_sound.as_ref().is_some_and(|p| p.by == *by) {
        app.viewer_sound = None;
        app.speaker.stop(Channel::Preview);
    }
}

/// What a sound viewer's button did this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pressed {
    Play,
    Stop,
}

/// A sound's viewer in `room` (the hak editor's Preview's, the Hak
/// board's): its Play or Stop in the middle, its size under it and, for a
/// WAVE under .bmu, that the game does not play it. Play takes the Preview
/// channel from whatever plays on it. While it plays, the window looks
/// again every while, to say Play once it ends. What was pressed.
pub(crate) fn sound_ui(
    app: &mut Moonglow,
    ui: &mut Ui,
    room: egui::Rect,
    by: Player,
    key: ResKey,
    data: &Arc<[u8]>,
) -> Option<Pressed> {
    let playing = plays(app, &by, key);
    if playing && let Some(p) = app.viewer_sound.as_mut() {
        p.pass = ui.ctx().cumulative_pass_nr();
    }
    let (icon, words, tip) = match playing {
        true => (mg_uikit::Icon::Stop, "Stop", "Stop the sound"),
        false => (mg_uikit::Icon::Play, "Play", "Play the sound"),
    };
    let b = mg_uikit::KitButton::new(Some(icon), words).tip(tip);
    let size = egui::vec2(b.width(ui), mg_uikit::metrics::CONTROL_HEIGHT);
    let place = egui::Rect::from_center_size(room.center() - egui::vec2(0.0, 20.0), size);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(place));
    let mut pressed = None;
    if child.add(b).clicked() {
        if playing {
            stop_by(app, &by);
            pressed = Some(Pressed::Stop);
        } else {
            let sound = crate::audio::Sound { name: key.resref, data: data.clone() };
            app.speaker.play(Channel::Preview, sound, 1.0, false);
            let pass = ui.ctx().cumulative_pass_nr();
            app.viewer_sound = Some(Playing { by, key, pass });
            pressed = Some(Pressed::Play);
        }
    }
    let t = mg_uikit::tokens(ui.ctx());
    let mut said = vec![format!("A sound, {}", crate::hak_view::size(data.len() as u64))];
    // (A RIFF file's chunks answer at once: an MP3's first frame would be
    // searched for.)
    let wave = data.starts_with(b"RIFF")
        && mg_audio::container(data).is_some_and(|(c, _)| c == mg_audio::Container::Wave);
    if key.restype == ResType::BMU && wave {
        said.push("The game does not play a WAVE under .bmu: music must be MP3".to_string());
    }
    let font = egui::FontId::proportional(12.0);
    let mut y = place.bottom() + 10.0;
    for (n, line) in said.into_iter().enumerate() {
        let galley = ui.painter().layout(line.clone(), font.clone(), t.text_3, room.width() - 20.0);
        let at = egui::pos2(room.center().x - galley.size().x / 2.0, y);
        let rect = egui::Rect::from_min_size(at, galley.size());
        y += galley.size().y + 2.0;
        ui.painter().galley(at, galley, t.text_3);
        // (Read by a screen reader as the words they are.)
        let r = ui.interact(rect, ui.id().with(("viewer-sound-said", n)), egui::Sense::hover());
        r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &line));
    }
    if app.viewer_sound.is_some() && (playing || pressed == Some(Pressed::Play)) {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
    }
    pressed
}

/// Each frame, as it begins: a viewer's sound that ended let go of, and
/// one whose viewer was not drawn in the last pass (its tab closed or
/// behind another, its panel folded or closed) stopped; the read-only
/// tabs' pictures not drawn for a while let go of.
pub(crate) fn follow(app: &mut Moonglow, ctx: &egui::Context) {
    app.decodes.shared.stored.store(false, Ordering::Relaxed);
    let pass = ctx.cumulative_pass_nr();
    if let Some(p) = &app.viewer_sound {
        if !app.speaker.playing(Channel::Preview) {
            app.viewer_sound = None;
        } else if p.pass + 1 < pass {
            let by = p.by.clone();
            stop_by(app, &by);
        }
    }
    crate::browser::let_go_of_pictures(app, ctx.input(|i| i.time));
}
