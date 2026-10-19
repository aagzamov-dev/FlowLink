use crate::model::{Display, Placement, Rect};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
}
#[derive(Debug, Clone)]
pub struct Edge {
    pub local_id: String,
    pub remote_id: String,
    pub side: Side,
    pub start: f64,
    pub end: f64,
    pub boundary: f64,
}

pub fn edges(local: &[Display], remote: &[Placement]) -> Vec<Edge> {
    let mut result = Vec::new();
    for screen in local.iter().filter(|d| d.online) {
        let a = &screen.rect;
        for p in remote {
            let b = &p.rect;
            for (side, gap, start, end, boundary) in [
                (
                    Side::Left,
                    (a.x - b.right()).abs(),
                    a.y.max(b.y),
                    a.bottom().min(b.bottom()),
                    a.x,
                ),
                (
                    Side::Right,
                    (a.right() - b.x).abs(),
                    a.y.max(b.y),
                    a.bottom().min(b.bottom()),
                    a.right(),
                ),
                (
                    Side::Top,
                    (a.y - b.bottom()).abs(),
                    a.x.max(b.x),
                    a.right().min(b.right()),
                    a.y,
                ),
                (
                    Side::Bottom,
                    (a.bottom() - b.y).abs(),
                    a.x.max(b.x),
                    a.right().min(b.right()),
                    a.bottom(),
                ),
            ] {
                if gap <= 1.0 && end - start >= 16.0 {
                    result.push(Edge {
                        local_id: screen.id.clone(),
                        remote_id: p.display_id.clone(),
                        side,
                        start,
                        end,
                        boundary,
                    });
                }
            }
        }
    }
    result
}
impl Edge {
    pub fn hit(&self, x: f64, y: f64) -> bool {
        let (along, cross) = match self.side {
            Side::Left | Side::Right => (y, x),
            _ => (x, y),
        };
        along > self.start + 2.0 && along < self.end - 2.0 && (cross - self.boundary).abs() <= 2.0
    }
    pub fn fraction(&self, x: f64, y: f64) -> f64 {
        let along = match self.side {
            Side::Left | Side::Right => y,
            _ => x,
        };
        ((along - self.start) / (self.end - self.start)).clamp(0.0, 1.0)
    }
    pub fn entry(&self, fraction: f64, remote_virtual: &Rect, actual: &Rect) -> (f64, f64) {
        let along = self.start + fraction.clamp(0.0, 1.0) * (self.end - self.start);
        match self.side {
            Side::Right => (
                actual.x + 3.0,
                actual.y + ((along - remote_virtual.y) / remote_virtual.height) * actual.height,
            ),
            Side::Left => (
                actual.right() - 4.0,
                actual.y + ((along - remote_virtual.y) / remote_virtual.height) * actual.height,
            ),
            Side::Bottom => (
                actual.x + ((along - remote_virtual.x) / remote_virtual.width) * actual.width,
                actual.y + 3.0,
            ),
            Side::Top => (
                actual.x + ((along - remote_virtual.x) / remote_virtual.width) * actual.width,
                actual.bottom() - 4.0,
            ),
        }
    }
}
pub fn overlaps(a: &Rect, b: &Rect) -> bool {
    a.x < b.right() - 1.0
        && a.right() > b.x + 1.0
        && a.y < b.bottom() - 1.0
        && a.bottom() > b.y + 1.0
}
pub fn validate_layout(local: &[Display], remote: &[Placement]) -> std::result::Result<(), String> {
    if remote.len() != 1 || !remote[0].rect.valid() {
        return Err("Place one Mac display beside your Windows displays".into());
    }
    if local.iter().any(|d| overlaps(&d.rect, &remote[0].rect)) {
        return Err("Displays cannot overlap".into());
    }
    if edges(local, remote).is_empty() {
        return Err("Snap the Mac to a shared edge of a Windows display".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn display(x: f64, y: f64, w: f64, h: f64) -> Display {
        Display {
            id: "local".into(),
            device_id: "windows".into(),
            name: "Screen".into(),
            number: 1,
            rect: Rect {
                x,
                y,
                width: w,
                height: h,
            },
            pixel_width: w as u32,
            pixel_height: h as u32,
            scale: 1.0,
            rotation: 0.0,
            online: true,
        }
    }
    #[test]
    fn partial_edge_maps_to_actual_retina_points() {
        let local = display(0.0, 0.0, 1920.0, 1080.0);
        let p = Placement {
            display_id: "mac".into(),
            rect: Rect {
                x: 1920.0,
                y: 200.0,
                width: 1512.0,
                height: 982.0,
            },
        };
        let e = edges(&[local], std::slice::from_ref(&p));
        assert_eq!(e.len(), 1);
        let f = e[0].fraction(1920.0, 640.0);
        let actual = Rect {
            x: -1512.0,
            y: 0.0,
            width: 1512.0,
            height: 982.0,
        };
        let (_, y) = e[0].entry(f, &p.rect, &actual);
        assert!((y - 440.0).abs() < 0.01);
    }
    #[test]
    fn corners_do_not_switch() {
        let e = edges(
            &[display(0.0, 0.0, 100.0, 100.0)],
            &[Placement {
                display_id: "mac".into(),
                rect: Rect {
                    x: 100.0,
                    y: 100.0,
                    width: 100.0,
                    height: 100.0,
                },
            }],
        );
        assert!(e.is_empty());
    }
    #[test]
    fn negative_coordinates_and_above() {
        let d = display(-1920.0, 0.0, 1920.0, 1080.0);
        let p = Placement {
            display_id: "mac".into(),
            rect: Rect {
                x: -1000.0,
                y: -900.0,
                width: 1400.0,
                height: 900.0,
            },
        };
        let e = edges(&[d], &[p]);
        assert_eq!(e[0].side, Side::Top);
        assert!(e[0].hit(-500.0, 0.0));
    }
    #[test]
    fn overlap_rejected() {
        assert!(validate_layout(
            &[display(0.0, 0.0, 100.0, 100.0)],
            &[Placement {
                display_id: "m".into(),
                rect: Rect {
                    x: 50.0,
                    y: 0.0,
                    width: 100.0,
                    height: 100.0
                }
            }]
        )
        .is_err());
    }
}
