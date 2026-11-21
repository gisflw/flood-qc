pub(crate) struct InundationStages {
    above_z0: Vec<f32>,
    ranges: Vec<StageRange>,
}

struct StageRange {
    min: f32,
    max: f32,
    has_missing: bool,
}

impl InundationStages {
    pub(crate) fn new(depth_m: &[f32], channel_depth_m: &[f32]) -> Self {
        let mut ranges: Vec<_> = channel_depth_m
            .iter()
            .map(|_| StageRange {
                min: f32::INFINITY,
                max: f32::NEG_INFINITY,
                has_missing: false,
            })
            .collect();
        let mut above_z0 = Vec::with_capacity(depth_m.len());
        for depth in depth_m.chunks_exact(channel_depth_m.len()) {
            for ((depth, channel_depth), range) in
                depth.iter().zip(channel_depth_m).zip(&mut ranges)
            {
                let stage = depth - channel_depth;
                if stage.is_finite() {
                    range.min = range.min.min(stage);
                    range.max = range.max.max(stage);
                } else {
                    range.has_missing = true;
                }
                above_z0.push(stage);
            }
        }
        Self { above_z0, ranges }
    }

    pub(crate) fn frames(&self) -> impl Iterator<Item = &[f32]> {
        self.above_z0.chunks_exact(self.ranges.len())
    }

    /// Initialize the tile and retain the cells whose classification can change.
    pub(crate) fn initialize_tile(
        &self,
        cells: &[Option<(f64, usize)>],
        mask: &mut Vec<u8>,
        changing: &mut Vec<ChangingCell>,
    ) -> bool {
        mask.clear();
        changing.clear();
        let first = &self.above_z0[..self.ranges.len()];
        let mut has_coverage = false;
        for (index, cell) in cells.iter().enumerate() {
            let Some((hand, mini)) = *cell else {
                mask.push(255);
                continue;
            };
            let range = &self.ranges[mini];
            if range.max == f32::NEG_INFINITY {
                mask.push(255);
                continue;
            }
            has_coverage = true;
            let may_change =
                range.has_missing || (f64::from(range.max) > hand && f64::from(range.min) <= hand);
            if may_change {
                changing.push(ChangingCell { index, hand, mini });
            }
            mask.push(mask_value(first[mini], hand));
        }
        has_coverage
    }
}

pub(crate) struct ChangingCell {
    index: usize,
    hand: f64,
    mini: usize,
}

pub(crate) fn update_tile(changing: &[ChangingCell], stages: &[f32], mask: &mut [u8]) -> bool {
    let mut changed = false;
    for cell in changing {
        let value = mask_value(stages[cell.mini], cell.hand);
        changed |= mask[cell.index] != value;
        mask[cell.index] = value;
    }
    changed
}

fn mask_value(stage: f32, hand: f64) -> u8 {
    if stage.is_finite() {
        u8::from(f64::from(stage) > hand)
    } else {
        255
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
    use super::{InundationStages, update_tile, wet_fraction};

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
        let stages = InundationStages::new(&[3., 3.], &[2., 1.]);
        let mut mask = Vec::new();
        let mut changing = Vec::new();
        assert!(stages.initialize_tile(&cells, &mut mask, &mut changing));
        assert!(changing.is_empty());
        assert_eq!(mask, [0, 1, 255, 255, 1, 255]);
        let mut overview = Vec::new();
        wet_fraction(&mask, 3, 2, 2, &mut overview);
        assert_eq!(overview, [50, 255]);
        wet_fraction(&mask, 3, 2, 4, &mut overview);
        assert_eq!(overview, [33]);
    }

    #[test]
    fn updates_crossings_and_missing_stages_and_reuses_tile_buffers() {
        let cells = [
            Some((1., 0)),
            Some((2., 0)),
            Some((0.5, 0)),
            None,
            Some((1., 1)),
            Some((1., 2)),
        ];
        let stages = InundationStages::new(
            &[
                3.,
                f32::NAN,
                f32::NAN,
                4.,
                3.,
                f32::INFINITY,
                4.,
                3.,
                f32::NAN,
                3.,
                f32::NAN,
                f32::INFINITY,
            ],
            &[2., 1., 1.],
        );
        let mut mask = Vec::new();
        let mut changing = Vec::new();
        assert!(stages.initialize_tile(&cells, &mut mask, &mut changing));
        assert_eq!(changing.len(), 2);
        assert_eq!(mask, [0, 0, 1, 255, 255, 255]);
        let mut frames = stages.frames().skip(1);
        assert!(update_tile(&changing, frames.next().unwrap(), &mut mask));
        assert_eq!(mask, [1, 0, 1, 255, 1, 255]);
        let unchanged = !update_tile(&changing, frames.next().unwrap(), &mut mask);
        assert!(unchanged);
        assert!(update_tile(&changing, frames.next().unwrap(), &mut mask));
        assert_eq!(mask, [0, 0, 1, 255, 255, 255]);

        let all_excluded =
            !stages.initialize_tile(&[None, Some((1., 2))], &mut mask, &mut changing);
        assert!(all_excluded);
        assert!(changing.is_empty());
        assert_eq!(mask, [255, 255]);
    }
}
