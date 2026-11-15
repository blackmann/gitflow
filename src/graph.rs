//! Commit graph layout. Commits arrive newest first; each row records which lanes
//! pass straight through, which lanes join into the commit's node from above, and
//! which lanes leave the node towards its parents below. Rows are painted
//! independently, so the graph virtualizes with the commit list.
//!
//! Lanes are reused once free but never shift sideways, so a line only bends at a
//! commit, the way Zed's and most desktop git graphs read.
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seg {
    /// A lane passing through this row without touching the commit.
    Pass { lane: usize, color: usize },
    /// A lane from the top of the row ending at this commit's node.
    Join { from: usize, color: usize },
    /// A line from this commit's node to the bottom of a lane, towards a parent.
    Leave { to: usize, color: usize },
}

#[derive(Clone, Debug)]
pub struct Row {
    pub lane: usize,
    pub color: usize,
    pub merge: bool,
    pub segs: Vec<Seg>,
}

pub struct Graph {
    pub rows: Vec<Row>,
    pub lanes: usize,
}

/// Lay out commits given as (sha, parent shas), newest first.
pub fn layout<'a>(commits: impl IntoIterator<Item = (&'a str, &'a [String])>) -> Graph {
    // Each lane waits for the commit it leads to, with the lane's color.
    let mut lanes: Vec<Option<(&str, usize)>> = vec![];
    let mut next_color = 0usize;
    let mut fresh = || {
        let c = next_color;
        next_color += 1;
        c
    };
    let mut rows = vec![];
    let mut widest = 0;
    for (sha, parents) in commits {
        let waiting = lanes
            .iter()
            .enumerate()
            .filter(|(_, l)| l.is_some_and(|(s, _)| s == sha))
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        let (lane, color) = match waiting.first() {
            Some(&i) => (i, lanes[i].unwrap().1),
            None => {
                let i = lanes.iter().position(Option::is_none).unwrap_or_else(|| {
                    lanes.push(None);
                    lanes.len() - 1
                });
                (i, fresh())
            }
        };
        let before = lanes.len();
        let mut segs = vec![];
        for (i, l) in lanes.iter().enumerate() {
            match l {
                Some((s, c)) if *s == sha => segs.push(Seg::Join { from: i, color: *c }),
                Some((_, c)) => segs.push(Seg::Pass { lane: i, color: *c }),
                None => {}
            }
        }
        for i in waiting {
            lanes[i] = None;
        }
        for (k, parent) in parents.iter().enumerate() {
            let parent = parent.as_str();
            if k == 0 {
                lanes[lane] = Some((parent, color));
                segs.push(Seg::Leave { to: lane, color });
                continue;
            }
            // A merge's other parent: join a lane already heading there, or open one.
            if let Some(i) = lanes.iter().position(|l| l.is_some_and(|(s, _)| s == parent)) {
                segs.push(Seg::Leave {
                    to: i,
                    color: lanes[i].unwrap().1,
                });
            } else {
                let i = lanes.iter().position(Option::is_none).unwrap_or_else(|| {
                    lanes.push(None);
                    lanes.len() - 1
                });
                let c = fresh();
                lanes[i] = Some((parent, c));
                segs.push(Seg::Leave { to: i, color: c });
            }
        }
        let width = before.max(lanes.len()).max(lane + 1);
        widest = widest.max(width);
        while lanes.last().is_some_and(Option::is_none) {
            lanes.pop();
        }
        rows.push(Row {
            lane,
            color,
            merge: parents.len() > 1,
            segs,
        });
    }
    Graph {
        rows,
        lanes: widest,
    }
}

/// Branch tips by commit, for decorating remote-only history.
pub fn tips(branches: &[(String, String)]) -> HashMap<String, Vec<String>> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for (name, sha) in branches {
        out.entry(sha.clone()).or_default().push(name.clone());
    }
    out
}

pub mod paint {
    use super::{Row, Seg};
    use gpui::*;

    pub const LANE: f32 = 14.;
    pub const PAD: f32 = 10.;

    pub fn width(lanes: usize) -> f32 {
        PAD * 2. + LANE * lanes.max(1) as f32
    }

    /// Paint one row into `bounds`. `colors` maps a color index to an RGB value.
    pub fn row(
        row: &Row,
        bounds: Bounds<Pixels>,
        colors: &dyn Fn(usize) -> u32,
        node_fill: u32,
        head: bool,
        window: &mut Window,
    ) {
        let x = |lane: usize| bounds.origin.x + px(PAD + LANE * lane as f32 + LANE / 2.);
        let top = bounds.origin.y;
        let bottom = bounds.origin.y + bounds.size.height;
        let mid = top + bounds.size.height / 2.;
        let cx = x(row.lane);
        let stroke = |from: Point<Pixels>, to: Point<Pixels>, color: u32, window: &mut Window| {
            let mut b = PathBuilder::stroke(px(1.6));
            b.move_to(from);
            if from.x == to.x {
                b.line_to(to);
            } else {
                // Leave vertically, bend, and arrive vertically.
                let dy = (to.y - from.y) * 0.62;
                b.cubic_bezier_to(
                    to,
                    point(from.x, from.y + dy),
                    point(to.x, to.y - dy),
                );
            }
            if let Ok(path) = b.build() {
                window.paint_path(path, rgb(color));
            }
        };
        for seg in &row.segs {
            match *seg {
                Seg::Pass { lane, color } => {
                    stroke(point(x(lane), top), point(x(lane), bottom), colors(color), window)
                }
                Seg::Join { from, color } => {
                    stroke(point(x(from), top), point(cx, mid), colors(color), window)
                }
                Seg::Leave { to, color } => {
                    stroke(point(cx, mid), point(x(to), bottom), colors(color), window)
                }
            }
        }
        let color = rgb(colors(row.color));
        let r = if row.merge { 3.5 } else { 4.5 };
        let node = Bounds::new(point(cx - px(r), mid - px(r)), size(px(r * 2.), px(r * 2.)));
        if head {
            let ring = 7.5;
            window.paint_quad(
                outline(
                    Bounds::new(
                        point(cx - px(ring), mid - px(ring)),
                        size(px(ring * 2.), px(ring * 2.)),
                    ),
                    color,
                    BorderStyle::Solid,
                )
                .corner_radii(px(ring)),
            );
        }
        if row.merge {
            // Merge commits read as hollow rings.
            window.paint_quad(fill(node, rgb(node_fill)).corner_radii(px(r)));
            window.paint_quad(outline(node, color, BorderStyle::Solid).corner_radii(px(r)));
        } else {
            window.paint_quad(fill(node, color).corner_radii(px(r)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(spec: &[(&str, &[&str])]) -> Graph {
        let owned = spec
            .iter()
            .map(|(s, p)| (s.to_string(), p.iter().map(|p| p.to_string()).collect::<Vec<_>>()))
            .collect::<Vec<_>>();
        layout(owned.iter().map(|(s, p)| (s.as_str(), p.as_slice())))
    }

    #[test]
    fn linear_history_stays_in_one_lane() {
        let g = graph(&[("c", &["b"]), ("b", &["a"]), ("a", &[])]);
        assert_eq!(g.lanes, 1);
        assert!(g.rows.iter().all(|r| r.lane == 0 && r.color == 0));
        assert_eq!(g.rows[2].segs, vec![Seg::Join { from: 0, color: 0 }]);
    }

    #[test]
    fn merges_open_and_close_a_side_lane() {
        // m merges f into main; f and b both descend from a.
        let g = graph(&[
            ("m", &["b", "f"]),
            ("f", &["a"]),
            ("b", &["a"]),
            ("a", &[]),
        ]);
        assert!(g.rows[0].merge);
        assert_eq!(g.rows[0].lane, 0);
        assert_eq!(g.rows[1].lane, 1, "the merged branch takes a side lane");
        assert_eq!(g.rows[2].lane, 0);
        assert_eq!(g.lanes, 2);
        // At a, both lanes join the root.
        let joins = g.rows[3]
            .segs
            .iter()
            .filter(|s| matches!(s, Seg::Join { .. }))
            .count();
        assert_eq!(joins, 2);
        assert_eq!(g.rows[3].lane, 0);
        // Side-lane color differs from main.
        assert_ne!(g.rows[1].color, g.rows[0].color);
    }

    #[test]
    fn separate_tips_get_their_own_lanes_and_free_lanes_are_reused() {
        let g = graph(&[
            ("x", &["a"]),
            ("y", &["a"]),
            ("a", &[]),
            ("z", &[]),
        ]);
        assert_eq!(g.rows[0].lane, 0);
        assert_eq!(g.rows[1].lane, 1);
        assert_eq!(g.rows[2].lane, 0);
        assert_eq!(g.rows[3].lane, 0, "a freed lane is reused");
    }

    #[test]
    fn second_parent_already_tracked_joins_that_lane() {
        let g = graph(&[
            ("t", &["p"]),
            ("m", &["q", "p"]),
            ("q", &["p"]),
            ("p", &[]),
        ]);
        let leaves = g.rows[1]
            .segs
            .iter()
            .filter_map(|s| match s {
                Seg::Leave { to, .. } => Some(*to),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(leaves, vec![1, 0], "m's second parent rides t's lane");
    }
}
