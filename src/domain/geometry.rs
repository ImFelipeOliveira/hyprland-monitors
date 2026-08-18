//! Pure layout geometry in logical pixels: edge snapping while dragging,
//! placement resolution on drop (no overlaps, no gaps, no vertex-only
//! neighbors), and normalization to a non-negative origin.

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
    fn x_overlap(&self, o: &Rect) -> i32 {
        self.right().min(o.right()) - self.x.max(o.x)
    }
    fn y_overlap(&self, o: &Rect) -> i32 {
        self.bottom().min(o.bottom()) - self.y.max(o.y)
    }
    /// Shares a positive-length edge segment. Touching only at a corner does
    /// not count: the cursor cannot cross a zero-length border.
    pub fn edge_adjacent(&self, o: &Rect) -> bool {
        ((self.right() == o.x || o.right() == self.x) && self.y_overlap(o) > 0)
            || ((self.bottom() == o.y || o.bottom() == self.y) && self.x_overlap(o) > 0)
    }
    fn center(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }
}

/// Minimum shared-edge length between flush neighbors: a quarter of the
/// smaller perpendicular extent, so the cursor always has a usable corridor.
fn min_edge_share(a: i32, b: i32) -> i32 {
    (a.min(b) / 4).max(1)
}

/// Range the sliding coordinate may take along a flush side of an obstacle
/// (start `o_start`, extent `o_extent`) while keeping the minimum edge share.
fn slide_bounds(dragged_extent: i32, o_start: i32, o_extent: i32) -> (i32, i32) {
    let share = min_edge_share(dragged_extent, o_extent);
    let lo = o_start - dragged_extent + share;
    let hi = o_start + o_extent - share;
    (lo, hi.max(lo))
}

/// Snap a dragged rect against `others`: flush against a side with the
/// sliding coordinate clamped to keep a real shared edge (never vertex-only),
/// preferring edge alignment when it is within `threshold`. The nearest
/// candidate whose per-axis displacement fits `threshold` wins; otherwise the
/// rect stays free.
pub fn snap(dragged: Rect, others: &[Rect], threshold: i32) -> (i32, i32) {
    let slide = |free: i32, (lo, hi): (i32, i32), aligns: [i32; 2]| -> i32 {
        aligns
            .into_iter()
            .filter(|a| (a - free).abs() <= threshold)
            .min_by_key(|a| (a - free).abs())
            .unwrap_or(free.clamp(lo, hi))
    };
    let mut best: Option<(i64, (i32, i32))> = None;
    for o in others {
        let y = slide(
            dragged.y,
            slide_bounds(dragged.h, o.y, o.h),
            [o.y, o.bottom() - dragged.h],
        );
        let x = slide(
            dragged.x,
            slide_bounds(dragged.w, o.x, o.w),
            [o.x, o.right() - dragged.w],
        );
        for (cx, cy) in [
            (o.x - dragged.w, y),
            (o.right(), y),
            (x, o.y - dragged.h),
            (x, o.bottom()),
        ] {
            let (dx, dy) = ((cx - dragged.x) as i64, (cy - dragged.y) as i64);
            if dx.abs() > threshold as i64 || dy.abs() > threshold as i64 {
                continue;
            }
            let d = dx * dx + dy * dy;
            if best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, (cx, cy)));
            }
        }
    }
    best.map_or((dragged.x, dragged.y), |(_, p)| p)
}

/// Flush placements against every side of `o`: the free coordinate clamped to
/// keep the minimum edge share, plus both edge-aligned variants.
fn placement_candidates(dragged: &Rect, o: &Rect) -> Vec<(i32, i32)> {
    let (lo_y, hi_y) = slide_bounds(dragged.h, o.y, o.h);
    let (lo_x, hi_x) = slide_bounds(dragged.w, o.x, o.w);
    let ys = [dragged.y.clamp(lo_y, hi_y), o.y, o.bottom() - dragged.h];
    let xs = [dragged.x.clamp(lo_x, hi_x), o.x, o.right() - dragged.w];
    let mut out = Vec::with_capacity(12);
    for y in ys {
        out.push((o.x - dragged.w, y));
        out.push((o.right(), y));
    }
    for x in xs {
        out.push((x, o.y - dragged.h));
        out.push((x, o.bottom()));
    }
    out
}

/// Ensure a valid placement for `dragged` among `others`: no overlap and, when
/// others exist, at least one shared edge (vertex-only contact and floating
/// gaps both fail). A position already satisfying both is kept; otherwise the
/// nearest flush placement wins. Falls back to flush right of everything.
pub fn resolve_placement(dragged: Rect, others: &[Rect]) -> (i32, i32) {
    if others.is_empty() {
        return (dragged.x, dragged.y);
    }
    let valid = |r: &Rect| {
        !others.iter().any(|o| r.overlaps(o)) && others.iter().any(|o| r.edge_adjacent(o))
    };
    if valid(&dragged) {
        return (dragged.x, dragged.y);
    }
    let (cx, cy) = dragged.center();
    others
        .iter()
        .flat_map(|o| placement_candidates(&dragged, o))
        .filter(|&(x, y)| valid(&Rect::new(x, y, dragged.w, dragged.h)))
        .min_by_key(|&(x, y)| {
            let (fx, fy) = Rect::new(x, y, dragged.w, dragged.h).center();
            let (dx, dy) = ((fx - cx) as i64, (fy - cy) as i64);
            dx * dx + dy * dy
        })
        .unwrap_or_else(|| {
            let rightmost = others.iter().max_by_key(|o| o.right()).unwrap();
            (rightmost.right(), rightmost.y)
        })
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
    fn snap_clamps_to_keep_shared_edge() {
        // Flush right but slid almost past the corner: pulled back so at least
        // a quarter of the smaller height stays shared (1080 - 270 = 810).
        let dragged = Rect::new(1910, 830, 1920, 1080);
        assert_eq!(snap(dragged, &[base()], 24), (1920, 810));
    }

    #[test]
    fn snap_never_produces_vertex_contact() {
        // Diagonally past the bottom-right corner: attaching would need a jump
        // beyond the threshold, so the rect stays free — never corner-glued.
        let dragged = Rect::new(1912, 1088, 1920, 1080);
        assert_eq!(snap(dragged, &[base()], 24), (1912, 1088));
    }

    #[test]
    fn edge_adjacency_requires_shared_edge() {
        let b = base();
        assert!(Rect::new(1920, 200, 1920, 1080).edge_adjacent(&b)); // flush right
        assert!(Rect::new(-500, 1080, 1920, 1080).edge_adjacent(&b)); // flush below
        assert!(!Rect::new(1920, 1080, 1920, 1080).edge_adjacent(&b)); // vertex only
        assert!(!Rect::new(1930, 0, 1920, 1080).edge_adjacent(&b)); // gap
    }

    #[test]
    fn overlap_resolves_to_nearest_free_side() {
        // Dropped mostly over the right half of base → pushed flush right.
        let dragged = Rect::new(1200, 100, 1920, 1080);
        let (x, y) = resolve_placement(dragged, &[base()]);
        let r = Rect::new(x, y, 1920, 1080);
        assert!(!r.overlaps(&base()));
        assert!(r.edge_adjacent(&base()));
        assert_eq!(x, 1920);
    }

    #[test]
    fn overlap_between_two_monitors_finds_free_spot() {
        let others = vec![base(), Rect::new(1920, 0, 1920, 1080)];
        let dragged = Rect::new(960, 200, 1920, 1080);
        let (x, y) = resolve_placement(dragged, &others);
        let r = Rect::new(x, y, 1920, 1080);
        assert!(others.iter().all(|o| !r.overlaps(o)));
        assert!(others.iter().any(|o| r.edge_adjacent(o)));
    }

    #[test]
    fn detached_drop_attaches_to_nearest_edge() {
        // Floating to the right with a gap → pulled flush against base.
        let dragged = Rect::new(2400, 300, 1920, 1080);
        assert_eq!(resolve_placement(dragged, &[base()]), (1920, 300));
    }

    #[test]
    fn vertex_contact_resolves_to_shared_edge() {
        // Touching base only at its bottom-right corner → slid up until a
        // quarter of the height is shared.
        let dragged = Rect::new(1920, 1080, 1920, 1080);
        let (x, y) = resolve_placement(dragged, &[base()]);
        let r = Rect::new(x, y, 1920, 1080);
        assert!(r.edge_adjacent(&base()));
        assert_eq!((x, y), (1920, 810));
    }

    #[test]
    fn no_overlap_keeps_position() {
        let dragged = Rect::new(1920, 0, 1920, 1080);
        assert_eq!(resolve_placement(dragged, &[base()]), (1920, 0));
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
