//! Pure layout geometry in logical pixels: edge snapping while dragging,
//! overlap resolution on drop, and normalization to a non-negative origin.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }
    pub fn right(&self) -> i32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }
    pub fn overlaps(&self, o: &Rect) -> bool {
        self.x < o.right() && o.x < self.right() && self.y < o.bottom() && o.y < self.bottom()
    }
    fn center(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }
}

/// Snap a dragged rect's position against the edges of `others`, per axis independently.
/// Candidates per axis: flush placement (left-of/right-of, above/below) and edge alignment.
/// The nearest candidate within `threshold` wins; otherwise the axis stays free.
pub fn snap(dragged: Rect, others: &[Rect], threshold: i32) -> (i32, i32) {
    let mut best_x: Option<(i32, i32)> = None; // (distance, value)
    let mut best_y: Option<(i32, i32)> = None;
    let consider = |best: &mut Option<(i32, i32)>, current: i32, candidate: i32| {
        let d = (candidate - current).abs();
        if d <= threshold && best.is_none_or(|(bd, _)| d < bd) {
            *best = Some((d, candidate));
        }
    };
    for o in others {
        for cx in [o.x - dragged.w, o.right(), o.x, o.right() - dragged.w] {
            consider(&mut best_x, dragged.x, cx);
        }
        for cy in [o.y - dragged.h, o.bottom(), o.y, o.bottom() - dragged.h] {
            consider(&mut best_y, dragged.y, cy);
        }
    }
    (
        best_x.map_or(dragged.x, |(_, v)| v),
        best_y.map_or(dragged.y, |(_, v)| v),
    )
}

/// If `dragged` overlaps any of `others`, move it to the nearest snapped placement
/// that overlaps nothing. Falls back to the far right of everything.
pub fn resolve_overlap(dragged: Rect, others: &[Rect]) -> (i32, i32) {
    if !others.iter().any(|o| dragged.overlaps(o)) {
        return (dragged.x, dragged.y);
    }
    let mut candidates: Vec<(i32, i32)> = Vec::new();
    for o in others {
        // Flush against each side, keeping the perpendicular coordinate...
        candidates.push((o.x - dragged.w, dragged.y));
        candidates.push((o.right(), dragged.y));
        candidates.push((dragged.x, o.y - dragged.h));
        candidates.push((dragged.x, o.bottom()));
        // ...and fully aligned to that side's corner.
        candidates.push((o.x - dragged.w, o.y));
        candidates.push((o.right(), o.y));
        candidates.push((o.x, o.y - dragged.h));
        candidates.push((o.x, o.bottom()));
    }
    let free: Vec<(i32, i32)> = candidates
        .into_iter()
        .filter(|&(x, y)| {
            let r = Rect::new(x, y, dragged.w, dragged.h);
            !others.iter().any(|o| r.overlaps(o))
        })
        .collect();
    if free.is_empty() {
        let max_right = others.iter().map(Rect::right).max().unwrap_or(0);
        return (max_right, dragged.y);
    }
    let (cx, cy) = dragged.center();
    free.into_iter()
        .min_by_key(|&(x, y)| {
            let r = Rect::new(x, y, dragged.w, dragged.h);
            let (fx, fy) = r.center();
            let (dx, dy) = ((fx - cx) as i64, (fy - cy) as i64);
            dx * dx + dy * dy
        })
        .unwrap()
}

/// Offset that shifts the layout so the bounding box of the rects at `anchor_indices`
/// (the enabled monitors) starts at 0x0. Apply to every monitor.
pub fn normalize_offset(rects: &[Rect], anchor_indices: &[usize]) -> (i32, i32) {
    let xs = anchor_indices
        .iter()
        .filter_map(|&i| rects.get(i))
        .map(|r| r.x);
    let ys = anchor_indices
        .iter()
        .filter_map(|&i| rects.get(i))
        .map(|r| r.y);
    let min_x = xs.min().unwrap_or(0);
    let min_y = ys.min().unwrap_or(0);
    (-min_x, -min_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Rect {
        // eDP-1-like reference monitor at origin.
        Rect::new(0, 0, 1920, 1080)
    }

    #[test]
    fn snaps_flush_to_the_right() {
        let dragged = Rect::new(1900, 12, 1920, 1080);
        let (x, y) = snap(dragged, &[base()], 24);
        assert_eq!((x, y), (1920, 0)); // flush right edge, top-aligned
    }

    #[test]
    fn snaps_flush_to_the_left() {
        let dragged = Rect::new(-1910, -8, 1920, 1080);
        let (x, y) = snap(dragged, &[base()], 24);
        assert_eq!((x, y), (-1920, 0));
    }

    #[test]
    fn snaps_above_and_below() {
        let above = Rect::new(5, -1090, 1920, 1080);
        assert_eq!(snap(above, &[base()], 24), (0, -1080));
        let below = Rect::new(-13, 1071, 1920, 1080);
        assert_eq!(snap(below, &[base()], 24), (0, 1080));
    }

    #[test]
    fn no_snap_outside_threshold() {
        let dragged = Rect::new(2400, 500, 1920, 1080);
        assert_eq!(snap(dragged, &[base()], 24), (2400, 500));
    }

    #[test]
    fn overlap_resolves_to_nearest_free_side() {
        // Dropped mostly over the right half of base → pushed flush right.
        let dragged = Rect::new(1200, 100, 1920, 1080);
        let (x, y) = resolve_overlap(dragged, &[base()]);
        let r = Rect::new(x, y, 1920, 1080);
        assert!(!r.overlaps(&base()));
        assert_eq!(x, 1920);
    }

    #[test]
    fn overlap_between_two_monitors_finds_free_spot() {
        let others = vec![base(), Rect::new(1920, 0, 1920, 1080)];
        let dragged = Rect::new(960, 200, 1920, 1080);
        let (x, y) = resolve_overlap(dragged, &others);
        let r = Rect::new(x, y, 1920, 1080);
        assert!(others.iter().all(|o| !r.overlaps(o)));
    }

    #[test]
    fn no_overlap_keeps_position() {
        let dragged = Rect::new(1920, 0, 1920, 1080);
        assert_eq!(resolve_overlap(dragged, &[base()]), (1920, 0));
    }

    #[test]
    fn normalize_uses_only_anchor_monitors() {
        let rects = vec![
            Rect::new(-1920, 0, 1920, 1080),   // enabled
            Rect::new(0, -500, 1920, 1080),    // enabled
            Rect::new(-5000, -5000, 800, 600), // disabled, ignored
        ];
        assert_eq!(normalize_offset(&rects, &[0, 1]), (1920, 500));
    }

    #[test]
    fn single_monitor_normalizes_to_origin() {
        let rects = vec![Rect::new(320, 240, 1920, 1080)];
        assert_eq!(normalize_offset(&rects, &[0]), (-320, -240));
    }
}
