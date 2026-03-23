// Snap guide engine — detects edge alignment between panels during drag

use super::config::SNAP_THRESHOLD;
use egui::{Rect, Vec2};

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
        return SnapResult {
            delta,
            guides: Vec::new(),
        };
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
                    let adjustment = ox - mx;
                    let better = snap_dx.is_none_or(|(best_adjustment, _)| {
                        adjustment.abs() < best_adjustment.abs()
                    });
                    if better {
                        snap_dx = Some((adjustment, ox));
                    }
                }
            }
        }

        // Y-axis snapping
        for &my in &moving_ys {
            for &oy in &other_ys {
                let dist = (oy - my).abs();
                if dist < SNAP_THRESHOLD {
                    let adjustment = oy - my;
                    let better = snap_dy.is_none_or(|(best_adjustment, _)| {
                        adjustment.abs() < best_adjustment.abs()
                    });
                    if better {
                        snap_dy = Some((adjustment, oy));
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

    SnapResult {
        delta: adjusted_delta,
        guides,
    }
}

/// Compute snapped resize delta and guide lines.
/// `panel` = the panel being resized (current rect).
/// `others` = rects of all other panels.
/// `delta` = raw resize delta.
/// `resize_left` = true if resizing from left edge.
pub fn snap_resize(panel: Rect, others: &[Rect], delta: Vec2, resize_left: bool) -> SnapResult {
    if others.is_empty() {
        return SnapResult {
            delta,
            guides: Vec::new(),
        };
    }

    let mut snap_dx: Option<(f32, f32)> = None;
    let mut snap_dy: Option<(f32, f32)> = None;
    let mut guides = Vec::new();

    // Which edges are moving?
    let moving_x = if delta.x.abs() > f32::EPSILON {
        Some(if resize_left {
            panel.min.x + delta.x
        } else {
            panel.max.x + delta.x
        })
    } else {
        None
    };
    let moving_y = if delta.y.abs() > f32::EPSILON {
        Some(panel.max.y + delta.y)
    } else {
        None
    };

    for other in others {
        if let Some(mx) = moving_x {
            for &ox in &[other.min.x, other.max.x] {
                let dist = (ox - mx).abs();
                if dist < SNAP_THRESHOLD {
                    let adj = ox - mx;
                    if snap_dx.is_none_or(|(best, _)| adj.abs() < best.abs()) {
                        snap_dx = Some((adj, ox));
                    }
                }
            }
        }

        if let Some(my) = moving_y {
            for &oy in &[other.min.y, other.max.y] {
                let dist = (oy - my).abs();
                if dist < SNAP_THRESHOLD {
                    let adj = oy - my;
                    if snap_dy.is_none_or(|(best, _)| adj.abs() < best.abs()) {
                        snap_dy = Some((adj, oy));
                    }
                }
            }
        }
    }

    let mut adjusted_delta = delta;

    if let Some((adj, snap_x)) = snap_dx {
        adjusted_delta.x += adj;
        let mut min_y = panel.min.y;
        let mut max_y = panel.max.y + adjusted_delta.y;
        for other in others {
            if [other.min.x, other.max.x]
                .iter()
                .any(|&ox| (ox - snap_x).abs() < 1.0)
            {
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

    if let Some((adj, snap_y)) = snap_dy {
        adjusted_delta.y += adj;
        let mut min_x = if resize_left {
            panel.min.x + adjusted_delta.x
        } else {
            panel.min.x
        };
        let mut max_x = if resize_left {
            panel.max.x
        } else {
            panel.max.x + adjusted_delta.x
        };
        for other in others {
            if [other.min.y, other.max.y]
                .iter()
                .any(|&oy| (oy - snap_y).abs() < 1.0)
            {
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

    SnapResult {
        delta: adjusted_delta,
        guides,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Pos2;

    #[test]
    fn picks_the_closest_snap_candidate() {
        let moving = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 100.0));
        let others = [
            Rect::from_min_max(Pos2::new(6.0, 20.0), Pos2::new(106.0, 120.0)),
            Rect::from_min_max(Pos2::new(3.0, 4.0), Pos2::new(103.0, 104.0)),
        ];

        let result = snap_drag(moving, &others, Vec2::ZERO);

        assert_eq!(result.delta, Vec2::new(3.0, 4.0));
        assert_eq!(result.guides.len(), 2);
    }
}
