//! The shapes between two placed blocks, on positions alone: a line along the longest axis, a
//! wall standing on the longer horizontal axis, a floor at the first block's height. Nearest the
//! first block first, at most a cap.

pub type Pos = [i32; 3];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Off,
    Line,
    Wall,
    Floor,
}

impl Mode {
    pub const ALL: [Mode; 4] = [Mode::Off, Mode::Line, Mode::Wall, Mode::Floor];

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    /// Its line in the language files (`builder.mode.<key>`).
    pub fn key(self) -> &'static str {
        match self {
            Mode::Off => "off",
            Mode::Line => "line",
            Mode::Wall => "wall",
            Mode::Floor => "floor",
        }
    }
}

fn span(a: i32, b: i32) -> impl Iterator<Item = i32> {
    let step = if b >= a { 1 } else { -1 };
    (0..=(b - a).abs()).map(move |k| a + k * step)
}

/// Every position of `mode`'s shape from `a` to `b`, both included, nearest `a` first, at most
/// `cap`. Off is no shape.
pub fn fill(mode: Mode, a: Pos, b: Pos, cap: usize) -> Vec<Pos> {
    let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let mut out: Vec<Pos> = match mode {
        Mode::Off => Vec::new(),
        Mode::Line => {
            let axis = (0..3).max_by_key(|&i| (d[i].abs(), std::cmp::Reverse(i))).unwrap_or(0);
            span(a[axis], b[axis])
                .map(|v| {
                    let mut p = a;
                    p[axis] = v;
                    p
                })
                .collect()
        }
        Mode::Wall => {
            let h = if d[0].abs() >= d[2].abs() { 0 } else { 2 };
            span(a[1], b[1])
                .flat_map(|y| {
                    span(a[h], b[h]).map(move |v| {
                        let mut p = [a[0], y, a[2]];
                        p[h] = v;
                        p
                    })
                })
                .collect()
        }
        Mode::Floor => span(a[0], b[0]).flat_map(|x| span(a[2], b[2]).map(move |z| [x, a[1], z])).collect(),
    };
    out.sort_by_key(|p| (0..3).map(|i| (p[i] - a[i]).abs()).sum::<i32>());
    out.truncate(cap);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_cycle_through_off() {
        assert_eq!(Mode::Off.next(), Mode::Line);
        assert_eq!(Mode::Floor.next(), Mode::Off);
    }

    #[test]
    fn a_line_follows_the_longest_axis() {
        assert_eq!(fill(Mode::Line, [0, 0, 0], [3, 1, 0], 64), vec![[0, 0, 0], [1, 0, 0], [2, 0, 0], [3, 0, 0]]);
        assert_eq!(fill(Mode::Line, [0, 5, 0], [0, 5, -2], 64), vec![[0, 5, 0], [0, 5, -1], [0, 5, -2]]);
    }

    #[test]
    fn a_wall_stands_on_the_longer_horizontal_axis() {
        let w = fill(Mode::Wall, [0, 0, 0], [2, 1, 1], 64);
        assert_eq!(w.len(), 6);
        assert!(w.iter().all(|p| p[2] == 0));
        assert!(w.contains(&[2, 1, 0]));
    }

    #[test]
    fn a_floor_keeps_the_first_height() {
        let f = fill(Mode::Floor, [0, 4, 0], [-1, 9, 2], 64);
        assert_eq!(f.len(), 6);
        assert!(f.iter().all(|p| p[1] == 4));
    }

    #[test]
    fn the_cap_keeps_the_nearest_and_off_is_nothing() {
        let f = fill(Mode::Floor, [0, 0, 0], [9, 0, 9], 10);
        assert_eq!(f.len(), 10);
        assert_eq!(f[0], [0, 0, 0]);
        assert!(f.iter().all(|p| p[0] + p[2] <= 3));
        assert!(fill(Mode::Off, [0, 0, 0], [5, 0, 0], 64).is_empty());
    }
}
