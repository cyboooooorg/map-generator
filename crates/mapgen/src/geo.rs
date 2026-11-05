//! Geometry of the equirectangular grid: pixel sizes in kilometres,
//! neighbourhoods that wrap around in longitude, and distance fields.
use rayon::prelude::*;
use std::f64::consts::PI;

/// The pixel grid of a world, with the physical size of each pixel.
#[derive(Clone, Copy, Debug)]
pub struct Grid {
    pub width: u32,
    pub height: u32,
    pub circumference_km: f32,
}

impl Grid {
    pub fn pixel_count(&self) -> usize {
        (self.width * self.height) as usize
    }

    pub fn index(&self, x: u32, y: u32) -> usize {
        (y * self.width + x) as usize
    }

    /// Latitude of row `y` in radians: `+π/2` at row 0 (north), `-π/2` at the bottom.
    pub fn latitude(&self, y: u32) -> f64 {
        PI / 2.0 - (y as f64 + 0.5) / self.height as f64 * PI
    }

    /// East-west size of a pixel on row `y`, in km. Shrinks towards the poles.
    pub fn dx_km(&self, y: u32) -> f32 {
        // Floor at a tenth of a pixel so polar rows keep a non-zero width.
        let factor = self.latitude(y).cos().max(0.1) as f32;
        self.circumference_km * factor / self.width as f32
    }

    /// North-south size of a pixel, in km. Constant over the map.
    pub fn dy_km(&self) -> f32 {
        self.circumference_km / 2.0 / self.height as f32
    }

    /// The row-major indices of the same neighbours as
    /// [`Grid::neighbours`] (8, wrapping in longitude, none off the poles),
    /// without their distances: for the passes that only need which pixels
    /// touch.
    #[inline]
    pub fn neighbour_indices(&self, i: usize) -> impl Iterator<Item = usize> {
        let (w, h) = (self.width as usize, self.height as usize);
        let (x, y) = (i % w, i / w);
        let (west, east) = (
            if x == 0 { w - 1 } else { x - 1 },
            if x + 1 == w { 0 } else { x + 1 },
        );
        let rows = [
            (y > 0).then(|| y - 1),
            Some(y),
            (y + 1 < h).then_some(y + 1),
        ];
        rows.into_iter().enumerate().flat_map(move |(k, row)| {
            row.into_iter().flat_map(move |ny| {
                let base = ny * w;
                // The centre of the middle row is the pixel itself.
                let columns: &[usize] = if k == 1 { &[0, 2] } else { &[0, 1, 2] };
                columns.iter().map(move |&c| base + [west, x, east][c])
            })
        })
    }

    /// The 8 neighbours of `(x, y)` with their distance in km. Longitude
    /// wraps around; rows beyond the poles are skipped.
    pub fn neighbours(&self, x: u32, y: u32) -> impl Iterator<Item = (u32, u32, f32)> + '_ {
        let (w, h) = (self.width as i64, self.height as i64);
        let dy = self.dy_km();
        // East-west sizes of the rows above, at and below `y` (0 off the map).
        let row_dx = [-1i64, 0, 1].map(|oy| {
            let ny = y as i64 + oy;
            if ny < 0 || ny >= h {
                0.0
            } else {
                self.dx_km(ny as u32)
            }
        });
        (-1i64..=1).flat_map(move |oy| {
            (-1i64..=1).filter_map(move |ox| {
                if ox == 0 && oy == 0 {
                    return None;
                }
                let ny = y as i64 + oy;
                if ny < 0 || ny >= h {
                    return None;
                }
                let nx = (x as i64 + ox).rem_euclid(w);
                // Average the east-west size of both rows for diagonal steps.
                let dx = (row_dx[1] + row_dx[(oy + 1) as usize]) / 2.0;
                let dist = ((ox as f32 * dx).powi(2) + (oy as f32 * dy).powi(2)).sqrt();
                Some((nx as u32, ny as u32, dist))
            })
        })
    }

    /// Distance in km from every pixel to the nearest pixel where `source` is
    /// true (0 for sources), along 8-connected paths. Pixels further than
    /// `max_km` get `max_km`.
    ///
    /// A Dijkstra on a bucket queue (Dial's algorithm): buckets narrower
    /// than the shortest step, so a pixel's neighbours always land in a
    /// later bucket and every pixel of a bucket is final when the bucket is
    /// reached. Each pixel thus gets the same minimum over its neighbours as
    /// with a binary heap, bit for bit, without its log-time pops. Only as
    /// many buckets as one step spans are kept, reused in turn.
    pub fn distance_to(&self, source: &[bool], max_km: f32) -> Vec<f32> {
        let mut dist: Vec<f32> = source
            .par_iter()
            .map(|&s| if s { 0.0 } else { max_km })
            .collect();
        // The shortest and longest steps: an east-west one on the narrowest
        // row, a diagonal on the widest.
        let dy = self.dy_km();
        let (narrowest, widest) = (0..self.height)
            .map(|y| self.dx_km(y))
            .fold((dy, 0.0f32), |(lo, hi), dx| (lo.min(dx), hi.max(dx)));
        // A little narrower than the shortest step, for the rounding of
        // the sums.
        let width = 0.99 * narrowest;
        let longest = (widest * widest + dy * dy).sqrt();
        let ring = (longest / width).ceil() as usize + 2;
        let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); ring];
        let bucket_of = |d: f32| (d / width) as usize;
        // Only sources next to a non-source seed the search: the others
        // have nothing left to reach.
        buckets[0] = (0..self.pixel_count())
            .into_par_iter()
            .filter(|&i| {
                source[i] && {
                    let (x, y) = (i as u32 % self.width, i as u32 / self.width);
                    self.neighbours(x, y)
                        .any(|(nx, ny, _)| !source[self.index(nx, ny)])
                }
            })
            .map(|i| i as u32)
            .collect();
        let mut pending = buckets[0].len();
        let mut current = 0usize;
        while pending > 0 {
            let mut bucket = std::mem::take(&mut buckets[current % ring]);
            pending -= bucket.len();
            for &i in &bucket {
                let i = i as usize;
                let d = dist[i];
                // A pixel pushed again with a shorter distance is reached
                // from its later bucket only.
                if bucket_of(d) != current {
                    continue;
                }
                let (x, y) = (i as u32 % self.width, i as u32 / self.width);
                for (nx, ny, step) in self.neighbours(x, y) {
                    let j = self.index(nx, ny);
                    let nd = d + step;
                    if nd < dist[j] {
                        dist[j] = nd;
                        buckets[bucket_of(nd) % ring].push(j as u32);
                        pending += 1;
                    }
                }
            }
            // Keep its allocation for the turn of the ring that reuses it.
            bucket.clear();
            buckets[current % ring] = bucket;
            current += 1;
        }
        dist
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> Grid {
        Grid {
            width: 40,
            height: 20,
            circumference_km: 40_000.0,
        }
    }

    #[test]
    fn pixels_shrink_towards_the_poles() {
        let g = grid();
        assert!((g.dx_km(9) - 1000.0).abs() < 10.0);
        assert!(g.dx_km(0) < g.dx_km(5));
        assert_eq!(g.dy_km(), 1000.0);
    }

    #[test]
    fn neighbour_indices_match_neighbours() {
        let g = grid();
        for i in 0..g.pixel_count() {
            let (x, y) = ((i as u32) % g.width, (i as u32) / g.width);
            let mut a: Vec<usize> = g
                .neighbours(x, y)
                .map(|(nx, ny, _)| g.index(nx, ny))
                .collect();
            let mut b: Vec<usize> = g.neighbour_indices(i).collect();
            a.sort_unstable();
            b.sort_unstable();
            assert_eq!(a, b, "pixel {i}");
        }
    }

    #[test]
    fn neighbours_wrap_in_longitude() {
        let g = grid();
        let n: Vec<_> = g.neighbours(0, 10).collect();
        assert_eq!(n.len(), 8);
        assert!(n.iter().any(|&(x, _, _)| x == 39));
        assert_eq!(g.neighbours(5, 0).count(), 5);
    }

    #[test]
    fn distance_grows_away_from_sources_and_wraps() {
        let g = grid();
        let mut source = vec![false; g.pixel_count()];
        source[g.index(0, 10)] = true;
        let d = g.distance_to(&source, f32::MAX);
        assert_eq!(d[g.index(0, 10)], 0.0);
        // Two pixels east and two pixels west (across the seam) are equally far.
        assert!((d[g.index(2, 10)] - d[g.index(38, 10)]).abs() < 1e-3);
        assert!(d[g.index(5, 10)] > d[g.index(2, 10)]);
        let capped = g.distance_to(&source, 1500.0);
        assert_eq!(capped[g.index(20, 10)], 1500.0);
    }
}
