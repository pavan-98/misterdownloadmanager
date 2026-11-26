//! The main window in a phone's layout (Android, on a screen narrower than [`PHONE_WIDTH`]): one
//! column, the download list first. A bar at the top names the view and opens the categories (a
//! drawer) and the rest of the commands (a menu); the list's rows are tall enough for a finger,
//! each with its name, its progress and what it is doing; a bar at the bottom holds the commands
//! used most. Everything a command does is the desktop's ([`App::run`]): only the layout differs.
//!
//! `MDM_PHONE=1` lays the main window out so on a desktop too, to look at it (a test copy's
//! window, as `MDM_ZOOM`).

use super::*;

/// Narrower than this (in points), the main window takes the phone's layout.
pub const PHONE_WIDTH: f32 = 600.0;
/// The top and bottom bars' height: Android's 48 dp touch target and its margin.
const BAR_H: f32 = 56.0;
/// A row of the list: two lines of text and a progress bar, with room for a finger.
const ROW_H: f32 = 72.0;
/// The space around the bars' and rows' contents.
const PAD: f32 = 16.0;

/// The width a sheet's contents can use ([`sheet`]): the window's.
pub fn sheet_width(ctx: &egui::Context) -> f32 {
    ctx.content_rect().width()
}

/// The height a sheet's contents can use ([`sheet`]): the window's, less its top bar.
pub fn sheet_height(ctx: &egui::Context) -> f32 {
    ctx.content_rect().height() - BAR_H
}

/// A window on a phone (a dialog, a download's progress): a page over the whole main window,
/// as Android's apps show one, with its title in a top bar whose × closes it as the desktop
/// window's × does. It isn't an egui window: those are placed and framed by egui, can be dragged,
/// and give a screen reader's root to whichever was drawn last. `body` lays out the page under
/// the bar, in its own width ([`sheet_width`]). Returns its answer, and whether × was tapped.
pub(crate) fn sheet<R>(
    ctx: &egui::Context,
    id: egui::Id,
    title: &str,
    bg: egui::Color32,
    body: impl FnOnce(&mut Ui) -> R,
) -> (R, bool) {
    let room = ctx.content_rect();
    // Opening, it comes in from the trailing side and fades in, as Android's pages do.
    let shown = entrance(ctx, id, 0.25);
    let from = room.width() * 0.3 * (1.0 - shown);
    ctx.set_transform_layer(
        egui::LayerId::new(egui::Order::Foreground, id),
        egui::emath::TSTransform::from_translation(egui::vec2(
            if winui::rtl() { -from } else { from },
            0.0,
        )),
    );
    egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fixed_pos(room.min)
        .movable(false)
        .constrain(false)
        .fade_in(true)
        .show(ctx, |ui| {
            ui.set_min_size(room.size());
            ui.set_max_size(room.size());
            let full = room;
            // The page takes the taps that miss its controls (the list under it gets none), and
            // is a window for a screen reader, named by its title.
            let page = ui.interact(full, id.with("page"), egui::Sense::click());
            let name = title.to_owned();
            page.widget_info(move || egui::WidgetInfo::labeled(egui::Role::Window, true, &name));
            ui.painter().rect_filled(full, 0.0, bg);
            let _surface = winui::Surface::enter(full.min.x, full.max.x);
            let top = egui::Rect::from_min_size(full.min, egui::vec2(full.width(), BAR_H));
            let p = ui.painter();
            p.rect_filled(winui::rect(top), 0.0, colors().menubar);
            p.hline(
                winui::rect(top).x_range(),
                top.max.y - 0.5,
                egui::Stroke::new(1.0, colors().toolbar_line),
            );
            let close_at = egui::Rect::from_min_size(top.min, egui::vec2(BAR_H, BAR_H));
            let closed = bar_button(ui, close_at, id.with("close"), Glyph::Close);
            let tx = close_at.max.x + 4.0;
            let shown = elide(ui, title, font_bold(18.0), top.max.x - PAD - tx);
            ui.painter().text(
                winui::rect(egui::Rect::from_min_size(
                    pos2(tx, top.center().y),
                    egui::Vec2::ZERO,
                ))
                .min,
                if winui::rtl() {
                    egui::Align2::RIGHT_CENTER
                } else {
                    egui::Align2::LEFT_CENTER
                },
                shown,
                font_bold(18.0),
                colors().text,
            );
            let below = egui::Rect::from_min_max(pos2(full.min.x, top.max.y), full.max);
            let mut rest = ui.new_child(egui::UiBuilder::new().max_rect(below));
            (body(&mut rest), closed)
        })
        .inner
}

/// How far the entrance of what `id` names has come, from 0 to 1, eased out (fast, then
/// settling) over `secs`: counted from the first pass it is drawn in after one it wasn't, so it
/// plays each time it opens. Asks for the frames it needs.
fn entrance(ctx: &egui::Context, id: egui::Id, secs: f64) -> f32 {
    let now = ctx.input(|i| i.time);
    let pass = ctx.cumulative_pass_nr();
    let key = id.with("entrance");
    let since = match ctx.data(|d| d.get_temp::<(f64, u64)>(key)) {
        Some((since, last)) if last + 1 >= pass => since,
        _ => now,
    };
    ctx.data_mut(|d| d.insert_temp(key, (since, pass)));
    let t = ((now - since) / secs).clamp(0.0, 1.0) as f32;
    if t < 1.0 {
        ctx.request_repaint();
    }
    1.0 - (1.0 - t).powi(3)
}

/// A big button's height on a phone's page ([`button_rects`]): 40 points, a finger's.
pub(crate) const BUTTON_H: f32 = 40.0;

/// Where a phone's page puts its `n` buttons, from English y `top` of a page `width` wide (both
/// relative to the page's origin `o`, as `winui::r` takes them): big buttons across the page,
/// the first alone on its row when `lead` (the page's main action), the rest two by two. The
/// screen rects in the buttons' order, and the height the rows take.
pub(crate) fn button_rects(
    o: Pos2,
    top: f32,
    width: f32,
    n: usize,
    lead: bool,
) -> (Vec<egui::Rect>, f32) {
    const MARGIN: f32 = 11.0;
    const GAP: f32 = 8.0;
    let full = width - 2.0 * MARGIN;
    let half = (full - GAP) / 2.0;
    let mut rects = Vec::with_capacity(n);
    let mut y = top;
    let mut k = 0;
    if lead && n > 0 {
        rects.push(winui::r(o, MARGIN, y, full, BUTTON_H));
        y += BUTTON_H + GAP;
        k = 1;
    }
    while k < n {
        if k + 1 < n {
            rects.push(winui::r(o, MARGIN, y, half, BUTTON_H));
            rects.push(winui::r(o, MARGIN + half + GAP, y, half, BUTTON_H));
            k += 2;
        } else {
            rects.push(winui::r(o, MARGIN, y, full, BUTTON_H));
            k += 1;
        }
        y += BUTTON_H + GAP;
    }
    (rects, (y - GAP - top).max(0.0))
}

/// The width a window lays itself out in: the phone's page's on a phone (its controls take the
/// room it has), `desktop` elsewhere.
pub(crate) fn layout_width(ctx: &egui::Context, desktop: f32) -> f32 {
    if active(ctx) {
        sheet_width(ctx)
    } else {
        desktop
    }
}

/// A page laid out wider than the phone (a desktop window's layout: Download video, the
/// Scheduler...) drawn smaller to fit its width, so every control and button is on the screen:
/// in a layer of its own, scaled (input too) from the top left of `ui`'s room. `width` is the
/// layout's; `body` gets a room that wide, and as much taller as it was made smaller.
pub(crate) fn fitted<R>(
    ui: &mut Ui,
    id: egui::Id,
    width: f32,
    body: impl FnOnce(&mut Ui) -> R,
) -> R {
    let view = ui.max_rect();
    let s = (view.width() / width).min(1.0);
    if s > 0.995 {
        return body(ui);
    }
    let layer = egui::LayerId::new(ui.layer_id().order, id.with("fitted"));
    let parent = ui
        .ctx()
        .layer_transform_to_global(ui.layer_id())
        .unwrap_or(egui::emath::TSTransform::IDENTITY);
    let fit = egui::emath::TSTransform::new(view.min.to_vec2() * (1.0 - s), s);
    let to_screen = parent * fit;
    ui.ctx().set_transform_layer(layer, to_screen);
    ui.ctx().set_sublayer(ui.layer_id(), layer);
    let room = egui::Rect::from_min_size(view.min, view.size() / s);
    let mut inner = ui.new_child(egui::UiBuilder::new().layer_id(layer).max_rect(room));
    inner.set_clip_rect(room);
    ui.ctx().data_mut(|d| d.insert_temp(fit_key(), to_screen));
    let out = body(&mut inner);
    ui.ctx()
        .data_mut(|d| d.remove::<egui::emath::TSTransform>(fit_key()));
    out
}

fn fit_key() -> egui::Id {
    egui::Id::unique("phone-fitted")
}

/// `p`, a point of the page being drawn, on the screen: scaled with it while it is drawn
/// smaller ([`fitted`]); for what is drawn in a layer of its own over it (a drop-down's list).
pub(crate) fn to_screen(ctx: &egui::Context, p: Pos2) -> Pos2 {
    ctx.data(|d| d.get_temp::<egui::emath::TSTransform>(fit_key()))
        .map_or(p, |t| t * p)
}

/// Whether the main window takes the phone's layout now.
pub fn active(ctx: &egui::Context) -> bool {
    (cfg!(target_os = "android") && ctx.content_rect().width() < PHONE_WIDTH)
        || std::env::var_os("MDM_PHONE").is_some()
}

fn colors() -> &'static crate::palette::Palette {
    crate::theme::colors()
}

/// The phone's text sizes: Android's body (16) and caption (13) sizes, in MDM's own font.
fn font(size: f32) -> egui::FontId {
    egui::FontId::new(size, winui::tahoma().family)
}

fn font_bold(size: f32) -> egui::FontId {
    egui::FontId::new(size, winui::tahoma_bold().family)
}

/// Where the drawer is (kept with the window's memory, not in the app's settings).
fn drawer_id() -> egui::Id {
    egui::Id::unique("phone-drawer")
}

impl App {
    /// The main window in the phone's layout ([`active`]).
    pub(super) fn paint_phone(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let full = ui.max_rect();
        let _surface = winui::Surface::enter(full.min.x, full.max.x);
        let mut chosen: Option<(Cmd, Option<u64>)> = None;
        ui.painter().rect_filled(full, 0.0, colors().list);

        let top = egui::Rect::from_min_size(full.min, egui::vec2(full.width(), BAR_H));
        let bottom = egui::Rect::from_min_max(pos2(full.min.x, full.max.y - BAR_H), full.max);
        let body =
            egui::Rect::from_min_max(pos2(full.min.x, top.max.y), pos2(full.max.x, bottom.min.y));

        // The list.
        let (ids, rows) = self.list_rows();
        self.selection.retain(&ids);
        // Selection mode (a long press starts it), as Android's lists have it; out of it nothing
        // stays picked: a tap opens, and the commands act on what they name.
        let mut picking = ctx
            .data(|d| d.get_temp::<bool>(picking_id()))
            .unwrap_or(false);
        if picking && self.selection.ids().is_empty() {
            picking = false;
        }
        if !picking && !self.selection.ids().is_empty() && self.popup.is_none() {
            self.selection.apply(crate::selection::Pick::Clear, &ids);
        }
        let (selected, _) = self.selection.rows(&ids);
        if self.list_view != self.view {
            self.list.scroll = 0.0;
            self.list_view = self.view;
        }
        // Each row's own button: Pause while it runs, Resume when it can go on.
        let states: Vec<RowState> = ids
            .iter()
            .map(|id| match self.items.iter().find(|i| i.id == *id) {
                Some(i) if i.live.is_some() => RowState::Running,
                Some(i) if i.can_start() => RowState::Stopped,
                _ => RowState::Done,
            })
            .collect();
        let mut list = ui.new_child(egui::UiBuilder::new().max_rect(body));
        let picked = phone_list(
            &mut list,
            &rows,
            &selected,
            picking,
            &states,
            &mut self.icons,
        );
        if let Some((i, held)) = picked.tapped {
            if picking {
                // A tap (or a long press) adds the row or takes it away.
                self.selection
                    .apply(crate::selection::Pick::Toggle(i), &ids);
            } else if held {
                // A long press picks the row and starts selection mode.
                self.selection.apply(crate::selection::Pick::Only(i), &ids);
                picking = true;
            } else {
                // A tap opens it, as Android's lists do: a download that isn't done shows its
                // details (its progress page), a finished one opens its file.
                self.open_item(ids[i]);
            }
        }
        if let Some((i, at)) = picked.menu {
            // A right-click (the desktop's preview): its menu, as on the desktop.
            if !selected.get(i).copied().unwrap_or(false) {
                self.selection.apply(crate::selection::Pick::Only(i), &ids);
            }
            self.open_popup_asked(Popup::Item(Some(ids[i])), at, &ctx);
        }
        if let Some((i, pause)) = picked.action {
            chosen = Some((if pause { Cmd::Stop } else { Cmd::Resume }, Some(ids[i])));
        }
        // Back ends selection mode (a menu or the drawer over it closes first).
        let overlay = self.popup.is_some()
            || ctx
                .data(|d| d.get_temp::<bool>(drawer_id()))
                .unwrap_or(false);
        if picking
            && !overlay
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            picking = false;
            self.selection.apply(crate::selection::Pick::Clear, &ids);
        }
        if ids.is_empty()
            && let Some(cmd) = self.empty_state(ui, winui::rect(body))
        {
            chosen = Some((cmd, None));
        }

        // The top bar: the categories, the view's name, the rest of the commands.
        let p = ui.painter();
        p.rect_filled(winui::rect(top), 0.0, colors().menubar);
        p.hline(
            winui::rect(top).x_range(),
            top.max.y - 0.5,
            egui::Stroke::new(1.0, colors().toolbar_line),
        );
        let side = BAR_H;
        let menu_at = egui::Rect::from_min_size(top.min, egui::vec2(side, side));
        let more_at =
            egui::Rect::from_min_size(pos2(top.max.x - side, top.min.y), egui::vec2(side, side));
        let all_at = more_at.translate(egui::vec2(-side, 0.0));
        let picked_n = self.selection.ids().len() as u64;
        let (lead, title) = if picking {
            (
                Glyph::Close,
                crate::i18n::tn(picked_n, "{n} selected", "{n} selected"),
            )
        } else {
            (Glyph::Menu, self.view_title())
        };
        if bar_button(ui, menu_at, ("phone-lead", picking), lead) {
            if picking {
                picking = false;
                self.selection.apply(crate::selection::Pick::Clear, &ids);
            } else {
                ctx.data_mut(|d| d.insert_temp(drawer_id(), true));
            }
        }
        let tx = menu_at.max.x + 4.0;
        let title_room = if picking { all_at.min.x } else { more_at.min.x } - PAD - tx;
        let title = elide(ui, &title, font_bold(18.0), title_room);
        ui.painter().text(
            winui::rect(egui::Rect::from_min_size(
                pos2(tx, top.center().y),
                egui::Vec2::ZERO,
            ))
            .min,
            if winui::rtl() {
                egui::Align2::RIGHT_CENTER
            } else {
                egui::Align2::LEFT_CENTER
            },
            title,
            font_bold(18.0),
            colors().text,
        );
        if picking && bar_button(ui, all_at, "phone-select-all", Glyph::SelectAll) {
            self.selection.apply(crate::selection::Pick::All, &ids);
        }
        if bar_button(ui, more_at, "phone-more", Glyph::More) {
            let at = winui::rect(more_at).left_bottom();
            // In selection mode, the picked downloads' menu (Open, Move, Properties...).
            // (A picked download's menu acts on every one picked.)
            let popup = if picking {
                Popup::Item(self.selection.ids().first().copied())
            } else {
                Popup::PhoneMore
            };
            self.open_popup_asked(popup, at, &ctx);
        }
        ctx.data_mut(|d| d.insert_temp(picking_id(), picking));

        // The bottom bar: the commands used most; in selection mode, those for the downloads
        // picked.
        let picked_items = self.chosen(None);
        let running = picked_items.iter().any(|i| i.live.is_some());
        let resumable = picked_items.iter().any(|i| i.can_start());
        let deletable = picked_items.iter().any(|i| i.live.is_none());
        let any_running = self.items.iter().any(|i| i.live.is_some());
        let buttons: Vec<(Icon, &str, bool, Cmd)> = if picking {
            vec![
                (
                    Icon::Resume,
                    crate::i18n::tc("toolbar", "Resume"),
                    resumable,
                    Cmd::Resume,
                ),
                (
                    Icon::Stop,
                    crate::i18n::tc("toolbar", "Stop"),
                    running,
                    Cmd::Stop,
                ),
                (
                    Icon::Delete,
                    crate::i18n::tc("toolbar", "Delete"),
                    deletable,
                    Cmd::Remove,
                ),
            ]
        } else {
            vec![
                (
                    Icon::AddUrl,
                    crate::i18n::tc("toolbar", "Add URL"),
                    true,
                    Cmd::AddUrl,
                ),
                (
                    Icon::HelperVideo,
                    crate::i18n::t("Video"),
                    true,
                    Cmd::VideoPage,
                ),
                (
                    Icon::StopAll,
                    crate::i18n::tc("toolbar", "Stop All"),
                    any_running,
                    Cmd::StopAll,
                ),
                (
                    Icon::Scheduler,
                    crate::i18n::tc("toolbar", "Scheduler"),
                    true,
                    Cmd::Scheduler,
                ),
            ]
        };
        let p = ui.painter();
        p.rect_filled(winui::rect(bottom), 0.0, colors().menubar);
        p.hline(
            winui::rect(bottom).x_range(),
            bottom.min.y + 0.5,
            egui::Stroke::new(1.0, colors().toolbar_line),
        );
        let w = bottom.width() / buttons.len() as f32;
        for (k, (icon, label, enabled, cmd)) in buttons.into_iter().enumerate() {
            let at = egui::Rect::from_min_size(
                pos2(bottom.min.x + w * k as f32, bottom.min.y),
                egui::vec2(w, BAR_H),
            );
            if bottom_button(ui, at, icon, label, enabled) {
                chosen = Some((cmd, None));
            }
        }

        // The categories, as a drawer over the list.
        if ctx
            .data(|d| d.get_temp::<bool>(drawer_id()))
            .unwrap_or(false)
            && let Some(view) = self.drawer(ui, full)
        {
            self.view = view;
        }

        // A row's menu, the "more" menu, a node's: the desktop's entries, in a sheet from the
        // bottom with rows a finger can hit ([`menu_sheet`]).
        if let Some((kind, _, serial)) = self.popup {
            let (entries, target) = self.popup_entries(kind);
            match menu_sheet(ui, full, &entries, serial) {
                PopupResult::Open | PopupResult::Neighbor(_) => {}
                PopupResult::Chosen(c) => {
                    chosen = Some((c, target));
                    self.popup = None;
                }
                PopupResult::Dismissed => self.popup = None,
            }
        }
        if let Some((c, target)) = chosen {
            self.run(c, target, &ctx);
        }
    }

    /// Whether the drawer, a menu sheet or selection mode is over the list (Back closes it).
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub(crate) fn phone_overlay(&self, ctx: &egui::Context) -> bool {
        self.popup.is_some()
            || ctx
                .data(|d| d.get_temp::<bool>(drawer_id()))
                .unwrap_or(false)
            || ctx
                .data(|d| d.get_temp::<bool>(picking_id()))
                .unwrap_or(false)
    }

    /// The view's name for the top bar: the category's, the queue's, or All Downloads.
    fn view_title(&self) -> String {
        self.tree_rows()
            .into_iter()
            .find(|(r, ..)| r.selected)
            .map_or_else(
                || crate::i18n::t("All Downloads").to_owned(),
                |(r, ..)| crate::i18n::t(&r.label).to_owned(),
            )
    }

    /// The categories and queues over the list, from the leading side; a tap on one shows it
    /// and closes the drawer, as a tap outside it or Back does. Returns the view chosen.
    fn drawer(&mut self, ui: &mut Ui, full: egui::Rect) -> Option<View> {
        let ctx = ui.ctx().clone();
        // Closed, a frame without it at once (as the menu sheet's, [`menu_sheet`]).
        let close = || {
            ctx.data_mut(|d| d.insert_temp(drawer_id(), false));
            ctx.request_repaint();
        };
        let width = (full.width() - BAR_H).min(320.0);
        // Opening, it slides in from the leading side as the rest dims.
        let shown = entrance(&ctx, drawer_id(), 0.22);
        let pane = egui::Rect::from_min_size(
            pos2(full.min.x - width * (1.0 - shown), full.min.y),
            egui::vec2(width, full.height()),
        );
        // The rest of the window, dimmed: a tap there closes the drawer.
        let scrim = ui.interact(full, egui::Id::unique("phone-scrim"), egui::Sense::click());
        ui.painter().rect_filled(
            full,
            0.0,
            egui::Color32::from_black_alpha((110.0 * shown) as u8),
        );
        let mut view = None;
        let pane_screen = winui::rect(pane);
        ui.painter().rect_filled(pane_screen, 0.0, colors().tree);
        let rows = self.tree_rows();
        let mut y = pane.min.y + 8.0;
        let mut on_pane = false;
        for (k, (row, node, branch)) in rows.iter().enumerate() {
            let at = egui::Rect::from_min_size(pos2(pane.min.x, y), egui::vec2(width, 48.0));
            y += 48.0;
            let r = ui.interact(
                winui::rect(at),
                egui::Id::unique(("phone-node", k)),
                egui::Sense::click(),
            );
            on_pane |= r.hovered();
            if row.selected {
                ui.painter().rect_filled(
                    winui::rect(at.shrink2(egui::vec2(8.0, 2.0))),
                    8.0,
                    colors().selection_inactive,
                );
            }
            let indent = PAD + 20.0 * row.level as f32;
            self.icons.paint(
                ui,
                row.icon.clone(),
                // An English point: the painter places it (right to left, from the right).
                pos2(at.min.x + indent, at.center().y - 8.0),
            );
            let label = crate::i18n::t(&row.label);
            ui.painter().text(
                winui::rect(egui::Rect::from_min_size(
                    pos2(at.min.x + indent + 28.0, at.center().y),
                    egui::Vec2::ZERO,
                ))
                .min,
                if winui::rtl() {
                    egui::Align2::RIGHT_CENTER
                } else {
                    egui::Align2::LEFT_CENTER
                },
                label,
                font(16.0),
                colors().text,
            );
            if r.clicked() {
                match node {
                    Some(v) => {
                        view = Some(*v);
                        close();
                    }
                    None => self.toggle(*branch),
                }
            }
            if r.secondary_clicked() || r.long_touched() {
                let at = r.interact_pointer_pos().unwrap_or(r.rect.center());
                if node.is_some() || *branch == Branch::Queues {
                    self.open_popup_asked(Popup::Node(*node), at, &ctx);
                }
            }
        }
        if scrim.clicked()
            && !on_pane
            && !scrim
                .interact_pointer_pos()
                .is_some_and(|p| pane_screen.contains(p))
        {
            close();
        }
        // Back closes it first (Esc on the desktop's keys).
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            close();
        }
        view
    }
}

/// What a touch on the list asked for.
#[derive(Default)]
struct ListTouch {
    /// A row tapped: its index, and whether it was held (a long press).
    tapped: Option<(usize, bool)>,
    /// A row's menu asked for (a right-click, in the desktop's preview), where.
    menu: Option<(usize, Pos2)>,
    /// A row's own button tapped: true for Pause, false for Resume.
    action: Option<(usize, bool)>,
}

/// What a row's own button does ([`phone_list`]).
#[derive(Clone, Copy, PartialEq)]
enum RowState {
    /// Downloading: Pause.
    Running,
    /// Stopped, failed or waiting: Resume.
    Stopped,
    /// Complete (or can't start): no button.
    Done,
}

/// Whether the list is in selection mode (kept with the window's memory).
fn picking_id() -> egui::Id {
    egui::Id::unique("phone-picking")
}

/// The download list, one row per download: its icon, its name, then its progress bar and a line
/// of what it is doing (size, speed, time left, status), in the list's own words ([`list::Row`]),
/// and at its end its own button (Pause, Resume; [`RowState`]). In selection mode (`picking`) a
/// check circle takes the icon's place and the picked rows are tinted.
fn phone_list(
    ui: &mut Ui,
    rows: &[list::Row],
    selected: &[bool],
    picking: bool,
    states: &[RowState],
    icons: &mut chrome::ShellIcons,
) -> ListTouch {
    let mut touch = ListTouch::default();
    let width = ui.max_rect().width();
    egui::ScrollArea::vertical()
        .id_salt("phone-list")
        .auto_shrink([false, false])
        .show_rows(ui, ROW_H, rows.len(), |ui, range| {
            for i in range {
                let row = &rows[i];
                let (at, r) =
                    ui.allocate_exact_size(egui::vec2(width, ROW_H), egui::Sense::click());
                let state = states.get(i).copied().unwrap_or(RowState::Done);
                // The row's own button, at its end (not in selection mode).
                let button = (!picking && state != RowState::Done).then(|| {
                    egui::Rect::from_min_size(
                        pos2(at.max.x - PAD - 40.0, at.center().y - 20.0),
                        egui::vec2(40.0, 40.0),
                    )
                });
                let picked = picking && selected.get(i).copied().unwrap_or(false);
                let p = ui.painter();
                if picked {
                    p.rect_filled(at, 0.0, colors().selection_inactive);
                } else if r.is_pointer_button_down_on() {
                    p.rect_filled(at, 0.0, colors().row_pressed);
                }
                let (ink, muted) = (colors().text, colors().muted);
                // English x, mirrored for a right-to-left interface.
                let x = |dx: f32| winui::mx(at.min.x + dx);
                let align = if winui::rtl() {
                    egui::Align2::RIGHT_CENTER
                } else {
                    egui::Align2::LEFT_CENTER
                };
                if picking {
                    check_circle(
                        ui,
                        winui::rect(egui::Rect::from_center_size(
                            pos2(at.min.x + PAD + 8.0, at.min.y + 22.0),
                            egui::vec2(20.0, 20.0),
                        )),
                        picked,
                    );
                } else {
                    // An English point: the painter places it (right to left, from the right).
                    icons.paint(ui, row.icon.clone(), pos2(at.min.x + PAD, at.min.y + 14.0));
                }
                let text_x = PAD + 16.0 + 12.0;
                let text_w = width - text_x - PAD - if button.is_some() { 48.0 } else { 0.0 };
                let name = elide(ui, &row.cells[0], font(16.0), text_w);
                ui.painter().text(
                    pos2(x(text_x), at.min.y + 22.0),
                    align,
                    name,
                    font(16.0),
                    ink,
                );
                // The progress bar, when the size is known and it isn't done.
                let bar = egui::Rect::from_min_size(
                    pos2(at.min.x + text_x, at.min.y + 38.0),
                    egui::vec2(text_w, 4.0),
                );
                if let Some(done) = row.progress.filter(|_| state != RowState::Done) {
                    let screen = winui::rect(bar);
                    ui.painter()
                        .rect_filled(screen, 2.0, colors().progress_track);
                    let filled = egui::Rect::from_min_size(
                        bar.min,
                        egui::vec2(bar.width() * done.clamp(0.0, 1.0) as f32, bar.height()),
                    );
                    ui.painter()
                        .rect_filled(winui::rect(filled), 2.0, colors().progress_bar);
                }
                let line = [
                    row.cells[list::STATUS].as_str(),
                    row.cells[2].as_str(),
                    row.cells[list::RATE].as_str(),
                    row.cells[4].as_str(),
                ]
                .into_iter()
                .filter(|s| !s.trim().is_empty())
                .collect::<Vec<_>>()
                .join("  ·  ");
                let line = elide(ui, &line, font(13.0), text_w);
                ui.painter().text(
                    pos2(x(text_x), at.min.y + 56.0),
                    align,
                    line,
                    font(13.0),
                    muted,
                );
                ui.painter().hline(
                    winui::rect(at.with_min_x(at.min.x + text_x)).x_range(),
                    at.max.y - 0.5,
                    egui::Stroke::new(1.0, colors().list_grid),
                );
                // Its own button, over the row (a tap on it isn't one on the row).
                let mut on_button = false;
                if let Some(b) = button {
                    let screen = winui::rect(b);
                    let br = ui.interact(screen, r.id.with("own"), egui::Sense::click());
                    on_button = br.hovered() || br.is_pointer_button_down_on();
                    row_button(ui, screen, state == RowState::Running, &br);
                    if br.clicked() {
                        touch.action = Some((i, state == RowState::Running));
                    }
                }
                let held = !on_button && held_long(ui, &r);
                // The release that ends a long press is no tap: it picked the row already.
                let after_hold = r.clicked()
                    && ui.data_mut(|d| d.remove_temp::<bool>(r.id.with("held-done")).is_some());
                if r.secondary_clicked() {
                    let p = r.interact_pointer_pos().unwrap_or(at.center());
                    touch.menu = Some((i, p));
                } else if held {
                    touch.tapped = Some((i, true));
                    ui.data_mut(|d| d.insert_temp(r.id.with("held-done"), true));
                } else if r.clicked() && !after_hold && !on_button {
                    touch.tapped = Some((i, false));
                }
            }
        });
    touch
}

/// Selection mode's check circle at `screen`: filled with a tick when `on`, a ring when not.
fn check_circle(ui: &Ui, screen: egui::Rect, on: bool) {
    let p = ui.painter();
    let c = screen.center();
    if on {
        p.circle_filled(c, 10.0, colors().selection);
        let tick = [
            c + egui::vec2(-4.5, 0.0),
            c + egui::vec2(-1.5, 3.5),
            c + egui::vec2(5.0, -3.5),
        ];
        p.line_segment(
            [tick[0], tick[1]],
            egui::Stroke::new(2.0, colors().selection_text),
        );
        p.line_segment(
            [tick[1], tick[2]],
            egui::Stroke::new(2.0, colors().selection_text),
        );
    } else {
        p.circle_stroke(c, 9.0, egui::Stroke::new(1.5, colors().muted));
    }
}

/// A row's own round button at `screen`: Pause (two bars) or Resume (a triangle).
fn row_button(ui: &Ui, screen: egui::Rect, pause: bool, r: &egui::Response) {
    let p = ui.painter();
    let c = screen.center();
    if r.is_pointer_button_down_on() {
        p.circle_filled(c, 20.0, colors().toolbar_pressed);
    }
    p.circle_stroke(c, 15.0, egui::Stroke::new(1.5, colors().progress_bar));
    let ink = colors().progress_bar;
    if pause {
        for dx in [-3.5, 3.5] {
            p.rect_filled(
                egui::Rect::from_center_size(c + egui::vec2(dx, 0.0), egui::vec2(3.0, 11.0)),
                1.0,
                ink,
            );
        }
    } else {
        // Pointing the way the text reads.
        let d = if winui::rtl() { -1.0 } else { 1.0 };
        p.add(egui::Shape::convex_polygon(
            vec![
                c + egui::vec2(-4.0 * d, -6.5),
                c + egui::vec2(6.5 * d, 0.0),
                c + egui::vec2(-4.0 * d, 6.5),
            ],
            ink,
            egui::Stroke::NONE,
        ));
    }
    winui::expose(
        r,
        egui::Role::Button,
        // i18n: verb: a download's own button in the phone's list
        if pause {
            crate::i18n::t("Pause")
        } else {
            crate::i18n::t("Resume")
        },
        true,
        None,
    );
}

/// How long a row is held still before its menu opens: Android's long press.
const LONG_PRESS: f64 = 0.5;

/// Whether `r` has just been held still for [`LONG_PRESS`] (true once per press). MDM's own,
/// not egui's `long_touched`: that needs the press reported as a touch and a frame drawn between
/// 0.8 s and the release, and neither held on the emulator; this one counts any press (a mouse's
/// too, in the desktop's preview) and asks for the frames it needs.
fn held_long(ui: &Ui, r: &egui::Response) -> bool {
    let key = r.id.with("held");
    if !r.is_pointer_button_down_on() {
        ui.data_mut(|d| d.remove::<(f64, bool)>(key));
        return false;
    }
    let now = ui.input(|i| i.time);
    let (start, fired) = ui
        .data(|d| d.get_temp::<(f64, bool)>(key))
        .unwrap_or((now, false));
    // Moved away from where it pressed: a scroll, not a long press.
    let moved = ui.input(|i| {
        i.pointer
            .press_origin()
            .zip(i.pointer.latest_pos())
            .is_some_and(|(a, b)| a.distance(b) > 8.0)
    });
    let fire = !fired && !moved && now - start >= LONG_PRESS;
    ui.data_mut(|d| d.insert_temp(key, (start, fired || fire || moved)));
    if !fired && !moved {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(50));
    }
    fire
}

/// `text` cut with "…" to fit `width` in `font`.
fn elide(ui: &Ui, text: &str, font: egui::FontId, width: f32) -> String {
    let fits = |s: &str| {
        ui.painter()
            .layout_no_wrap(s.to_owned(), font.clone(), egui::Color32::WHITE)
            .size()
            .x
            <= width
    };
    if fits(text) {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let (mut lo, mut hi) = (0, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let s: String = chars[..mid].iter().collect::<String>() + "…";
        if fits(&s) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    chars[..lo].iter().collect::<String>() + "…"
}

/// The top bar's buttons' glyphs, drawn (no font has them all).
#[derive(Clone, Copy)]
enum Glyph {
    /// Three lines: the categories.
    Menu,
    /// Three dots: the rest of the commands.
    More,
    /// A cross: closes the page ([`sheet`]).
    Close,
    /// A box with a tick in it: Select all (selection mode).
    SelectAll,
}

/// A square button of the top bar at English `at`; true when tapped.
fn bar_button(
    ui: &mut Ui,
    at: egui::Rect,
    id: impl std::hash::Hash + std::fmt::Debug,
    glyph: Glyph,
) -> bool {
    let screen = winui::rect(at);
    let r = ui.interact(screen, egui::Id::unique(id), egui::Sense::click());
    let p = ui.painter();
    if r.is_pointer_button_down_on() {
        p.circle_filled(screen.center(), 20.0, colors().toolbar_pressed);
    }
    let c = screen.center();
    let ink = colors().text;
    match glyph {
        Glyph::Menu => {
            for dy in [-6.0, 0.0, 6.0] {
                p.line_segment(
                    [c + egui::vec2(-9.0, dy), c + egui::vec2(9.0, dy)],
                    egui::Stroke::new(2.0, ink),
                );
            }
        }
        Glyph::More => {
            for dy in [-6.0, 0.0, 6.0] {
                p.circle_filled(c + egui::vec2(0.0, dy), 2.0, ink);
            }
        }
        Glyph::SelectAll => {
            let stroke = egui::Stroke::new(2.0, ink);
            p.rect_stroke(
                egui::Rect::from_center_size(c, egui::vec2(16.0, 16.0)),
                2.0,
                stroke,
                egui::StrokeKind::Middle,
            );
            p.line_segment(
                [c + egui::vec2(-4.0, 0.0), c + egui::vec2(-1.0, 3.5)],
                stroke,
            );
            p.line_segment(
                [c + egui::vec2(-1.0, 3.5), c + egui::vec2(4.5, -3.5)],
                stroke,
            );
        }
        Glyph::Close => {
            let stroke = egui::Stroke::new(2.0, ink);
            p.line_segment(
                [c + egui::vec2(-7.0, -7.0), c + egui::vec2(7.0, 7.0)],
                stroke,
            );
            p.line_segment(
                [c + egui::vec2(-7.0, 7.0), c + egui::vec2(7.0, -7.0)],
                stroke,
            );
        }
    }
    r.clicked()
}

/// A button of the bottom bar at English `at`: the toolbar's icon over its label; true when
/// tapped.
fn bottom_button(ui: &mut Ui, at: egui::Rect, icon: Icon, label: &str, enabled: bool) -> bool {
    let screen = winui::rect(at);
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let r = ui.interact(screen, egui::Id::unique(("phone-bottom", label)), sense);
    if r.is_pointer_button_down_on() {
        ui.painter()
            .rect_filled(screen.shrink(4.0), 12.0, colors().toolbar_pressed);
    }
    let img = egui::Rect::from_center_size(
        pos2(screen.center().x, screen.min.y + 18.0),
        egui::vec2(22.0, 22.0),
    );
    icon.image(enabled).paint_at(ui, img);
    ui.painter().text(
        pos2(screen.center().x, screen.min.y + 42.0),
        egui::Align2::CENTER_CENTER,
        label,
        font(12.0),
        if enabled {
            colors().text
        } else {
            colors().text_disabled
        },
    );
    enabled && r.clicked()
}

/// A menu row's height in the sheet: Android's list item.
const MENU_ROW_H: f32 = 48.0;

/// `entries` (a popup menu of the desktop's) as a sheet over the bottom of `full`, one row a
/// finger can hit for each, under a dimmed window: a tap runs an item, opens a submenu in the
/// sheet's place (a first row goes back), or outside the sheet closes it, as Back does. Which
/// submenu is open is kept for menu `serial`.
fn menu_sheet<C: Clone>(
    ui: &mut Ui,
    full: egui::Rect,
    entries: &[MenuEntry<C>],
    serial: u64,
) -> PopupResult<C> {
    let ctx = ui.ctx().clone();
    let path_id = egui::Id::unique(("phone-menu-path", serial));
    let mut path: Vec<usize> = ctx.data(|d| d.get_temp(path_id)).unwrap_or_default();
    // The entries of the open submenu, and its title.
    let mut here = entries;
    let mut title: Option<&str> = None;
    for &k in &path {
        match here.get(k) {
            Some(MenuEntry::Sub(label, _, children)) => {
                title = Some(label);
                here = children;
            }
            _ => break,
        }
    }
    let rows = here.len() + usize::from(title.is_some());
    let height = (rows as f32 * MENU_ROW_H + 16.0).min(full.height() * 0.85);
    // Opening, it rises from the bottom as the window behind it dims.
    let shown = entrance(&ctx, egui::Id::unique(("phone-menu", serial)), 0.22);
    let sheet = egui::Rect::from_min_max(pos2(full.min.x, full.max.y - height), full.max)
        .translate(egui::vec2(0.0, height * (1.0 - shown)));
    let scrim = ui.interact(
        full,
        egui::Id::unique(("phone-menu-scrim", serial)),
        egui::Sense::click(),
    );
    ui.painter().rect_filled(
        full,
        0.0,
        egui::Color32::from_black_alpha((110.0 * shown) as u8),
    );
    ui.painter().rect_filled(
        sheet,
        egui::CornerRadius {
            nw: 16,
            ne: 16,
            sw: 0,
            se: 0,
        },
        colors().menu,
    );
    let mut result = PopupResult::Open;
    let mut inside =
        ui.new_child(egui::UiBuilder::new().max_rect(sheet.shrink2(egui::vec2(0.0, 8.0))));
    egui::ScrollArea::vertical()
        .id_salt(("phone-menu", serial, path.len()))
        .auto_shrink([false, false])
        .show(&mut inside, |ui| {
            let width = ui.max_rect().width();
            if let Some(title) = title {
                let (at, r) =
                    ui.allocate_exact_size(egui::vec2(width, MENU_ROW_H), egui::Sense::click());
                menu_row(
                    ui,
                    at,
                    &format!("‹  {}", menu_label(title)),
                    true,
                    false,
                    false,
                    &r,
                );
                if r.clicked() {
                    path.pop();
                }
            }
            for (k, e) in here.iter().enumerate() {
                if let MenuEntry::Sep = e {
                    let (at, _) =
                        ui.allocate_exact_size(egui::vec2(width, 9.0), egui::Sense::hover());
                    ui.painter().hline(
                        at.shrink2(egui::vec2(PAD, 0.0)).x_range(),
                        at.center().y,
                        egui::Stroke::new(1.0, colors().menu_sep),
                    );
                    continue;
                }
                let (label, enabled, checked, sub) = match e {
                    MenuEntry::Item(l, _, en, ch) | MenuEntry::Radio(l, _, en, ch) => {
                        (*l, *en, *ch, false)
                    }
                    MenuEntry::Sub(l, en, _) => (*l, *en, false, true),
                    MenuEntry::Sep => unreachable!(),
                };
                let sense = if enabled {
                    egui::Sense::click()
                } else {
                    egui::Sense::hover()
                };
                let (at, r) = ui.allocate_exact_size(egui::vec2(width, MENU_ROW_H), sense);
                menu_row(ui, at, &menu_label(label), enabled, checked, sub, &r);
                if enabled && r.clicked() {
                    match e {
                        MenuEntry::Item(_, c, ..) | MenuEntry::Radio(_, c, ..) => {
                            result = PopupResult::Chosen(c.clone());
                        }
                        MenuEntry::Sub(..) => path.push(k),
                        MenuEntry::Sep => {}
                    }
                }
            }
        });
    let outside = scrim.clicked()
        && !scrim
            .interact_pointer_pos()
            .is_some_and(|p| sheet.contains(p));
    // Back: out of the submenu, else the menu closes.
    let back = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
    if back && !path.is_empty() {
        path.pop();
    } else if back || outside {
        result = PopupResult::Dismissed;
    }
    if matches!(result, PopupResult::Open) {
        ctx.data_mut(|d| d.insert_temp(path_id, path));
    } else {
        ctx.data_mut(|d| d.remove::<Vec<usize>>(path_id));
        // A frame without the sheet now: a touch is matched to the last frame's widgets, and
        // the sheet's backdrop would take the next one.
        ctx.request_repaint();
    }
    result
}

/// A menu's label in the interface language, without its access letter's `&` or the keyboard
/// shortcut some labels carry ("Move/Rename (Ctrl-M)"): a phone has no keyboard for it.
fn menu_label(label: &str) -> String {
    chrome::without_shortcut(&chrome::plain(crate::i18n::tc("menu", label)).0).to_owned()
}

/// One row of [`menu_sheet`] at `at` (screen): its label, a check mark when checked, an arrow
/// when it opens a submenu.
fn menu_row(
    ui: &Ui,
    at: egui::Rect,
    label: &str,
    enabled: bool,
    checked: bool,
    sub: bool,
    r: &egui::Response,
) {
    let p = ui.painter();
    if enabled && r.is_pointer_button_down_on() {
        p.rect_filled(at, 0.0, colors().menu_hot);
    }
    let ink = if enabled {
        colors().menu_text
    } else {
        colors().menu_text_disabled
    };
    let rtl = winui::rtl();
    let lead = if rtl { at.max.x - PAD } else { at.min.x + PAD };
    let trail = if rtl { at.min.x + PAD } else { at.max.x - PAD };
    let dir = if rtl { -1.0 } else { 1.0 };
    let text_x = lead + dir * 28.0;
    p.text(
        pos2(text_x, at.center().y),
        if rtl {
            egui::Align2::RIGHT_CENTER
        } else {
            egui::Align2::LEFT_CENTER
        },
        label,
        font(16.0),
        ink,
    );
    let stroke = egui::Stroke::new(2.0, ink);
    if checked {
        let c = pos2(lead + dir * 9.0, at.center().y);
        p.line_segment(
            [
                c + egui::vec2(-6.0 * dir, 0.0),
                c + egui::vec2(-2.0 * dir, 4.0),
            ],
            stroke,
        );
        p.line_segment(
            [
                c + egui::vec2(-2.0 * dir, 4.0),
                c + egui::vec2(6.0 * dir, -5.0),
            ],
            stroke,
        );
    }
    if sub {
        let c = pos2(trail - dir * 4.0, at.center().y);
        p.line_segment([c + egui::vec2(-4.0 * dir, -6.0), c], stroke);
        p.line_segment([c, c + egui::vec2(-4.0 * dir, 6.0)], stroke);
    }
}

/// A download as a phone's notification shows it (android.rs `Notices`): running, with how far
/// it is and its figures, or just completed.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Notice {
    pub id: u64,
    pub name: String,
    /// How far it is (0 to 100), when its size is known.
    pub percent: Option<u8>,
    /// Its figures as the list says them: "45% · 1.25 MB/sec · 2 min".
    pub line: String,
    /// The file, for a completed one's Open.
    pub path: String,
}

impl App {
    /// The downloads running now, for their notifications (android.rs).
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub(crate) fn running_notices(&self) -> Vec<Notice> {
        self.items
            .iter()
            .filter(|i| i.live.is_some())
            .map(|i| {
                let frac = i.fraction();
                let mut parts = Vec::new();
                if let Some(f) = frac {
                    parts.push(crate::i18n::percent(f, 0));
                }
                let rate = i.rate();
                if rate > 0.0 {
                    parts.push(model::rate(rate));
                }
                let left = i.time_left();
                if !left.is_empty() {
                    parts.push(left);
                }
                Notice {
                    id: i.id,
                    name: i.file_name.clone(),
                    percent: frac.map(|f| (f * 100.0).clamp(0.0, 100.0) as u8),
                    line: parts.join("  ·  "),
                    path: i.dest.display().to_string(),
                }
            })
            .collect()
    }

    /// Download `id`, when it has completed: for its "Download complete" notification.
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub(crate) fn completed_notice(&self, id: u64) -> Option<Notice> {
        self.items
            .iter()
            .find(|i| i.id == id && i.status == Status::Complete)
            .map(|i| Notice {
                id: i.id,
                name: i.file_name.clone(),
                percent: Some(100),
                line: model::size3(i.size.unwrap_or(i.downloaded)),
                path: i.dest.display().to_string(),
            })
    }
}
