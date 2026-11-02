//! Geometry of the equirectangular grid: pixel sizes in kilometres,
//! neighbourhoods that wrap around in longitude, and distance fields.
use rayon::prelude::*;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
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
    pub fn distance_to(&self, source: &[bool], max_km: f32) -> Vec<f32> {
        #[derive(PartialEq)]
        struct Node(f32, usize);
        impl Eq for Node {}
        impl Ord for Node {
            // Reversed: BinaryHeap is a max-heap, we want the closest first.
            fn cmp(&self, other: &Self) -> Ordering {
                other.0.total_cmp(&self.0)
            }
        }
        impl PartialOrd for Node {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        let mut dist: Vec<f32> = source
            .par_iter()
            .map(|&s| if s { 0.0 } else { max_km })
            .collect();
        // Only sources next to a non-source seed the search: the others
        // have nothing left to reach.
        let mut heap: BinaryHeap<Node> = (0..self.pixel_count())
            .into_par_iter()
            .filter(|&i| {
                source[i] && {
                    let (x, y) = (i as u32 % self.width, i as u32 / self.width);
                    self.neighbours(x, y)
                        .any(|(nx, ny, _)| !source[self.index(nx, ny)])
                }
            })
            .map(|i| Node(0.0, i))
            .collect::<Vec<_>>()
            .into();
        while let Some(Node(d, i)) = heap.pop() {
            if d > dist[i] {
                continue;
            }
            let (x, y) = (i as u32 % self.width, i as u32 / self.width);
            for (nx, ny, step) in self.neighbours(x, y) {
                let j = self.index(nx, ny);
                let nd = d + step;
                if nd < dist[j] {
                    dist[j] = nd;
                    heap.push(Node(nd, j));
                }
            }
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
