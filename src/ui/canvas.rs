//! The drag-and-drop canvas: monitors as proportional rectangles with live
//! edge snapping while dragging; drops are committed through the session.

use super::App;
use crate::domain::geometry::{Rect, snap};
use crate::domain::monitor::format_refresh;
use egui::{Align2, Color32, CornerRadius, FontId, Sense, Stroke, StrokeKind};

const SNAP_SCREEN_PX: f32 = 24.0;

pub(super) struct DragState {
    pub idx: usize,
    pub start: (i32, i32),
    pub acc: egui::Vec2,
    /// Snapped logical position currently shown (committed on release).
    pub live: (i32, i32),
}

impl App {
    // The index drives monitors, rects and self.drag at once — clearer than iterators here.
    #[allow(clippy::needless_range_loop)]
    pub(super) fn draw_canvas(&mut self, outer: &mut egui::Ui) {
        egui::CentralPanel::default().show(outer, |ui| {
            let Some(s) = self.session.as_mut() else {
                return;
            };
            let avail = ui.available_rect_before_wrap();
            let painter = ui.painter_at(avail);

            // Logical rects, with the dragged monitor at its live (snapped) position.
            let mut rects: Vec<Rect> = (0..s.monitors.len()).map(|i| s.logical_rect(i)).collect();
            if let Some(d) = &self.drag
                && d.idx < rects.len()
            {
                rects[d.idx].x = d.live.0;
                rects[d.idx].y = d.live.1;
            }

            // Fit-to-viewport transform.
            let min_x = rects.iter().map(|r| r.x).min().unwrap_or(0) as f32;
            let min_y = rects.iter().map(|r| r.y).min().unwrap_or(0) as f32;
            let max_x = rects.iter().map(Rect::right).max().unwrap_or(1) as f32;
            let max_y = rects.iter().map(Rect::bottom).max().unwrap_or(1) as f32;
            let (bw, bh) = ((max_x - min_x).max(1.0), (max_y - min_y).max(1.0));
            let zoom = ((avail.width() / bw).min(avail.height() / bh) * 0.8).min(0.5);
            let origin =
                avail.center() - egui::vec2((min_x + bw / 2.0) * zoom, (min_y + bh / 2.0) * zoom);
            let to_screen = |r: &Rect| {
                egui::Rect::from_min_size(
                    origin + egui::vec2(r.x as f32 * zoom, r.y as f32 * zoom),
                    egui::vec2(r.w as f32 * zoom, r.h as f32 * zoom),
                )
            };

            let mut finalize: Option<(usize, (i32, i32))> = None;
            for i in 0..s.monitors.len() {
                let screen_rect = to_screen(&rects[i]);
                let id = egui::Id::new(("monitor", s.monitors[i].name.clone()));
                let resp = ui.interact(screen_rect, id, Sense::click_and_drag());

                if resp.clicked() || resp.drag_started() {
                    self.selected = Some(i);
                }
                if resp.drag_started() {
                    let start = s.monitors[i].pos;
                    self.drag = Some(DragState {
                        idx: i,
                        start,
                        acc: egui::Vec2::ZERO,
                        live: start,
                    });
                }
                if resp.dragged()
                    && let Some(d) = self.drag.as_mut().filter(|d| d.idx == i)
                {
                    d.acc += resp.drag_delta();
                    let free = (
                        d.start.0 + (d.acc.x / zoom).round() as i32,
                        d.start.1 + (d.acc.y / zoom).round() as i32,
                    );
                    let (w, h) = s.monitors[i].logical_size();
                    let threshold = ((SNAP_SCREEN_PX / zoom) as i32).max(8);
                    let obstacles = s.obstacle_rects(i);
                    d.live = snap(Rect::new(free.0, free.1, w, h), &obstacles, threshold);
                    rects[i].x = d.live.0;
                    rects[i].y = d.live.1;
                }
                if resp.drag_stopped()
                    && let Some(d) = self.drag.take().filter(|d| d.idx == i)
                {
                    finalize = Some((i, d.live));
                }

                // Paint.
                let m = &s.monitors[i];
                let is_selected = self.selected == Some(i);
                let is_dragging = self.drag.as_ref().is_some_and(|d| d.idx == i);
                let fill = if !m.enabled {
                    Color32::from_gray(60)
                } else if is_selected || is_dragging {
                    Color32::from_rgb(45, 90, 140)
                } else {
                    Color32::from_rgb(50, 60, 75)
                };
                let stroke = if is_selected {
                    Stroke::new(2.0, Color32::from_rgb(120, 180, 255))
                } else {
                    Stroke::new(1.0, Color32::from_gray(140))
                };
                let screen_rect = to_screen(&rects[i]); // may have moved this frame
                painter.rect_filled(screen_rect, CornerRadius::same(4), fill);
                painter.rect_stroke(
                    screen_rect,
                    CornerRadius::same(4),
                    stroke,
                    StrokeKind::Inside,
                );

                let text_color = if m.enabled {
                    Color32::WHITE
                } else {
                    Color32::from_gray(150)
                };
                painter.text(
                    screen_rect.center() - egui::vec2(0.0, 12.0),
                    Align2::CENTER_CENTER,
                    &m.name,
                    FontId::proportional(16.0),
                    text_color,
                );
                let subtitle = if m.enabled {
                    format!(
                        "{}x{} @ {} Hz",
                        m.mode.width,
                        m.mode.height,
                        format_refresh(m.mode.refresh)
                    )
                } else {
                    "disabled".to_string()
                };
                painter.text(
                    screen_rect.center() + egui::vec2(0.0, 8.0),
                    Align2::CENTER_CENTER,
                    subtitle,
                    FontId::proportional(12.0),
                    text_color.gamma_multiply(0.8),
                );
            }
            if let Some((i, pos)) = finalize {
                s.finalize_drop(i, pos);
            }
        });
    }
}
