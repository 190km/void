// Snap guide engine — detects edge alignment between panels during drag

use egui::{Rect, Vec2};

const SNAP_THRESHOLD: f32 = 8.0;

pub struct SnapGuide {
    pub vertical: bool, // true = vertical line (X alignment), false = horizontal (Y alignment)
    pub position: f32,  // x for vertical, y for horizontal
    pub start: f32,
    pub end: f32,
}

pub struct SnapResult {
    pub delta: Vec2,
    pub guides: Vec<SnapGuide>,
}

/// Compute snapped drag delta and guide lines.
/// `moving` = the panel being dragged (current position before delta).
/// `others` = rects of all other panels.
/// `delta` = raw drag delta in canvas space.
pub fn snap_drag(moving: Rect, others: &[Rect], delta: Vec2) -> SnapResult {
    if others.is_empty() {
        return SnapResult { delta, guides: Vec::new() };
    }

    let proposed = moving.translate(delta);
    let mut snap_dx: Option<(f32, f32)> = None; // (adjustment, snapped_x)
    let mut snap_dy: Option<(f32, f32)> = None;
    let mut guides = Vec::new();

    let moving_xs = [proposed.min.x, proposed.center().x, proposed.max.x];
    let moving_ys = [proposed.min.y, proposed.center().y, proposed.max.y];

    for other in others {
        let other_xs = [other.min.x, other.center().x, other.max.x];
        let other_ys = [other.min.y, other.center().y, other.max.y];

        // X-axis snapping
        for &mx in &moving_xs {
            for &ox in &other_xs {
                let dist = (ox - mx).abs();
                if dist < SNAP_THRESHOLD {
                    let better = snap_dx.map_or(true, |(_, best)| (ox - mx).abs() < (best - mx + snap_dx.unwrap().0).abs());
                    if better {
                        snap_dx = Some((ox - mx, ox));
                    }
                }
            }
        }

        // Y-axis snapping
        for &my in &moving_ys {
            for &oy in &other_ys {
                let dist = (oy - my).abs();
                if dist < SNAP_THRESHOLD {
                    let better = snap_dy.map_or(true, |(_, best)| (oy - my).abs() < (best - my + snap_dy.unwrap().0).abs());
                    if better {
                        snap_dy = Some((oy - my, oy));
                    }
                }
            }
        }
    }

    let mut adjusted_delta = delta;

    // Apply X snap and generate vertical guide
    if let Some((adj, snap_x)) = snap_dx {
        adjusted_delta.x += adj;
        let snapped = moving.translate(Vec2::new(delta.x + adj, delta.y));
        // Find extents for the guide line
        let mut min_y = snapped.min.y;
        let mut max_y = snapped.max.y;
        for other in others {
            let other_xs = [other.min.x, other.center().x, other.max.x];
            if other_xs.iter().any(|&ox| (ox - snap_x).abs() < 1.0) {
                min_y = min_y.min(other.min.y);
                max_y = max_y.max(other.max.y);
            }
        }
        guides.push(SnapGuide {
            vertical: true,
            position: snap_x,
            start: min_y - 10.0,
            end: max_y + 10.0,
        });
    }

    // Apply Y snap and generate horizontal guide
    if let Some((adj, snap_y)) = snap_dy {
        adjusted_delta.y += adj;
        let snapped = moving.translate(Vec2::new(adjusted_delta.x, delta.y + adj));
        let mut min_x = snapped.min.x;
        let mut max_x = snapped.max.x;
        for other in others {
            let other_ys = [other.min.y, other.center().y, other.max.y];
            if other_ys.iter().any(|&oy| (oy - snap_y).abs() < 1.0) {
                min_x = min_x.min(other.min.x);
                max_x = max_x.max(other.max.x);
            }
        }
        guides.push(SnapGuide {
            vertical: false,
            position: snap_y,
            start: min_x - 10.0,
            end: max_x + 10.0,
        });
    }

    SnapResult { delta: adjusted_delta, guides }
}
