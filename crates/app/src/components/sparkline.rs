use crate::types::SparkPoint;

pub fn build_path(points: &[SparkPoint], width: f32, height: f32) -> String {
    if points.is_empty() {
        return String::new();
    }
    if points.len() == 1 {
        let y = height / 2.0;
        return format!("M 0 {y:.1} L {width:.1} {y:.1}");
    }

    let mut min = points[0].players;
    let mut max = points[0].players;
    for p in &points[1..] {
        if p.players < min { min = p.players; }
        if p.players > max { max = p.players; }
    }
    let range = (max - min).max(1) as f32;
    let n = points.len() as f32;

    let mut out = String::new();
    for (i, p) in points.iter().enumerate() {
        let x = (i as f32 / (n - 1.0)) * width;
        let y = height - ((p.players as f32 - min as f32) / range) * height;
        if i == 0 {
            out.push_str(&format!("M {x:.1} {y:.1}"));
        } else {
            out.push_str(&format!(" L {x:.1} {y:.1}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_points_yields_empty_path() {
        assert_eq!(build_path(&[], 100.0, 20.0), "");
    }

    #[test]
    fn single_point_renders_horizontal_segment() {
        let p = vec![SparkPoint { t_unix: 0, players: 10 }];
        let s = build_path(&p, 100.0, 20.0);
        assert!(s.starts_with("M 0"), "got {s}");
        assert!(s.contains(" L 100"), "got {s}");
    }

    #[test]
    fn two_points_at_same_height_render_a_line() {
        let p = vec![
            SparkPoint { t_unix: 0, players: 10 },
            SparkPoint { t_unix: 60, players: 10 },
        ];
        let s = build_path(&p, 100.0, 20.0);
        assert!(s.starts_with("M "), "got {s}");
        assert!(s.contains(" L "), "got {s}");
    }
}
