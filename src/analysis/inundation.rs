/// A positive HAND measurement paired with a known mini basin is eligible.
pub(crate) fn classify(
    cells: &[Option<(f64, usize)>],
    depth_m: &[f32],
    channel_depth_m: &[f32],
    mask: &mut [u8],
) {
    for (value, cell) in mask.iter_mut().zip(cells) {
        *value = cell.map_or(255, |(hand, mini)| {
            let above_z0 = depth_m[mini] - channel_depth_m[mini];
            if !above_z0.is_finite() {
                255
            } else {
                u8::from(f64::from(above_z0) > hand)
            }
        });
    }
}

/// Summarize actual pixels in each block, including excluded pixels in the denominator.
pub(crate) fn wet_fraction(
    mask: &[u8],
    width: usize,
    height: usize,
    factor: usize,
    overview: &mut Vec<u8>,
) {
    overview.clear();
    for y in (0..height).step_by(factor) {
        for x in (0..width).step_by(factor) {
            let bottom = (y + factor).min(height);
            let right = (x + factor).min(width);
            let mut wet = 0usize;
            let mut covered = false;
            for row in y..bottom {
                for pixel in &mask[row * width + x..row * width + right] {
                    wet += usize::from(*pixel == 1);
                    covered |= *pixel != 255;
                }
            }
            let count = (bottom - y) * (right - x);
            overview.push(if covered {
                ((100 * wet + count / 2) / count) as u8
            } else {
                255
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{classify, wet_fraction};

    #[test]
    fn compares_strictly_above_z0_and_summarizes_actual_edge_pixels() {
        let cells = [
            Some((1., 0)),
            Some((0.5, 0)),
            None,
            None,
            Some((1., 1)),
            None,
        ];
        let mut mask = [0; 6];
        classify(&cells, &[3., 3.], &[2., 1.], &mut mask);
        assert_eq!(mask, [0, 1, 255, 255, 1, 255]);
        let mut overview = Vec::new();
        wet_fraction(&mask, 3, 2, 2, &mut overview);
        assert_eq!(overview, [50, 255]);
        wet_fraction(&mask, 3, 2, 4, &mut overview);
        assert_eq!(overview, [33]);
    }
}
