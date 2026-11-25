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
/// The title bar egui draws over a window embedded in the phone's one window (a dialog's sheet).
pub const SHEET_TITLE_H: f32 = 24.0;

/// The width a sheet's contents can use: the window's, less the frame egui draws around a
/// window embedded in it.
pub fn sheet_width(ctx: &egui::Context) -> f32 {
    ctx.content_rect().width() - 16.0
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
        let (selected, _) = self.selection.rows(&ids);
        if self.list_view != self.view {
            self.list.scroll = 0.0;
            self.list_view = self.view;
        }
        let mut list = ui.new_child(egui::UiBuilder::new().max_rect(body));
        let picked = phone_list(&mut list, &rows, &selected, &mut self.icons);
        if let Some((i, held)) = picked.tapped {
            // A tap picks the row alone; while several are picked (Select all, a long press's
            // menu), it adds or takes away this one.
            let picking = selected.iter().filter(|s| **s).count() > 1;
            let pick = if picking && !held {
                crate::selection::Pick::Toggle(i)
            } else {
                crate::selection::Pick::Only(i)
            };
            self.selection.apply(pick, &ids);
        }
        if let Some((i, at)) = picked.menu {
            if !selected.get(i).copied().unwrap_or(false) {
                self.selection.apply(crate::selection::Pick::Only(i), &ids);
            }
            self.open_popup_asked(Popup::Item(Some(ids[i])), at, &ctx);
        }
        if let Some(i) = picked.opened {
            self.open_item(ids[i]);
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
        if bar_button(ui, menu_at, "phone-categories", Glyph::Menu) {
            ctx.data_mut(|d| d.insert_temp(drawer_id(), true));
        }
        let title = self.view_title();
        let tx = menu_at.max.x + 4.0;
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
        if bar_button(ui, more_at, "phone-more", Glyph::More) {
            let at = winui::rect(more_at).left_bottom();
            self.open_popup_asked(Popup::PhoneMore, at, &ctx);
        }

        // The bottom bar: the commands used most, for the downloads picked.
        let picked_items = self.chosen(None);
        let running = picked_items.iter().any(|i| i.live.is_some());
        let resumable = picked_items.iter().any(|i| i.can_start());
        let deletable = picked_items.iter().any(|i| i.live.is_none());
        let buttons = [
            (Icon::AddUrl, "Add URL", true, Cmd::AddUrl),
            // i18n: verb: continue the selected downloads
            (Icon::Resume, "Resume", resumable, Cmd::Resume),
            // i18n: verb: stop the selected downloads
            (Icon::Stop, "Stop", running, Cmd::Stop),
            // i18n: verb: delete the selected downloads
            (Icon::Delete, "Delete", deletable, Cmd::Remove),
        ];
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
            if bottom_button(ui, at, icon, crate::i18n::t(label), enabled) {
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
        let pane = egui::Rect::from_min_size(full.min, egui::vec2(width, full.height()));
        // The rest of the window, dimmed: a tap there closes the drawer.
        let scrim = ui.interact(full, egui::Id::unique("phone-scrim"), egui::Sense::click());
        ui.painter()
            .rect_filled(full, 0.0, egui::Color32::from_black_alpha(110));
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
                pos2(winui::mx(at.min.x + indent), at.center().y - 8.0),
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
    /// A row's menu asked for (a long press, a right-click), where.
    menu: Option<(usize, Pos2)>,
    /// A row opened (a double tap).
    opened: Option<usize>,
}

/// The download list, one row per download: its icon, its name, then its progress bar and a line
/// of what it is doing (size, speed, time left, status), in the list's own words ([`list::Row`]).
fn phone_list(
    ui: &mut Ui,
    rows: &[list::Row],
    selected: &[bool],
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
                let p = ui.painter();
                let picked = selected.get(i).copied().unwrap_or(false);
                if picked {
                    p.rect_filled(at, 0.0, colors().selection);
                } else if r.is_pointer_button_down_on() {
                    p.rect_filled(at, 0.0, colors().row_pressed);
                }
                let ink = if picked {
                    colors().selection_text
                } else {
                    colors().text
                };
                let muted = if picked {
                    colors().selection_muted
                } else {
                    colors().muted
                };
                // English x, mirrored for a right-to-left interface.
                let x = |dx: f32| winui::mx(at.min.x + dx);
                let align = if winui::rtl() {
                    egui::Align2::RIGHT_CENTER
                } else {
                    egui::Align2::LEFT_CENTER
                };
                icons.paint(
                    ui,
                    row.icon.clone(),
                    pos2(
                        x(PAD) - if winui::rtl() { 16.0 } else { 0.0 },
                        at.min.y + 14.0,
                    ),
                );
                let text_x = PAD + 16.0 + 12.0;
                let text_w = width - text_x - PAD;
                let name = elide(ui, &row.cells[0], font(16.0), text_w);
                ui.painter().text(
                    pos2(x(text_x), at.min.y + 22.0),
                    align,
                    name,
                    font(16.0),
                    ink,
                );
                // The progress bar, when the size is known.
                let bar = egui::Rect::from_min_size(
                    pos2(at.min.x + text_x, at.min.y + 38.0),
                    egui::vec2(text_w, 4.0),
                );
                if let Some(done) = row.progress {
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
                    at.x_range(),
                    at.max.y - 0.5,
                    egui::Stroke::new(1.0, colors().list_grid),
                );
                if r.double_clicked() {
                    touch.opened = Some(i);
                } else if r.secondary_clicked() || r.long_touched() {
                    let p = r.interact_pointer_pos().unwrap_or(at.center());
                    touch.menu = Some((i, p));
                    touch.tapped = Some((i, true));
                } else if r.clicked() {
                    touch.tapped = Some((i, false));
                }
            }
        });
    touch
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
}

/// A square button of the top bar at English `at`; true when tapped.
fn bar_button(ui: &mut Ui, at: egui::Rect, id: &str, glyph: Glyph) -> bool {
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
    let sheet = egui::Rect::from_min_max(pos2(full.min.x, full.max.y - height), full.max);
    let scrim = ui.interact(
        full,
        egui::Id::unique(("phone-menu-scrim", serial)),
        egui::Sense::click(),
    );
    ui.painter()
        .rect_filled(full, 0.0, egui::Color32::from_black_alpha(110));
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

/// A menu's label in the interface language, without its access letter's `&`.
fn menu_label(label: &str) -> String {
    chrome::plain(crate::i18n::tc("menu", label)).0
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
