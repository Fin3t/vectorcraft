//! The colour of fully clear pixels in an exported PNG. Their alpha stays 0, so the picture looks
//! the same; what changes is what filtering and mipmapping (a game engine's texture import) blend
//! into the edges: black by default, a chosen colour, or the colour of the nearest visible pixel.

use std::collections::VecDeque;

/// What fully clear pixels (alpha 0) of a PNG hold.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClearPixels {
    /// (0, 0, 0, 0), as before.
    #[default]
    Keep,
    /// This colour, alpha 0.
    Color([u8; 3]),
    /// The colour of the nearest pixel with any alpha (a breadth-first flood from them); an image
    /// without visible pixels stays as it is.
    Bleed,
}

/// Apply `clear` to straight RGBA pixels of a `width` × `height` image (`px` of another length
/// is left alone).
pub fn apply(px: &mut [u8], width: u32, height: u32, clear: ClearPixels) {
    let (w, h) = (width as usize, height as usize);
    let n = w.saturating_mul(h);
    if n == 0 || px.len() != n.saturating_mul(4) {
        return;
    }
    let pixels = px.as_chunks_mut::<4>().0;
    match clear {
        ClearPixels::Keep => {}
        ClearPixels::Color(c) => {
            for p in pixels.iter_mut().filter(|p| p[3] == 0) {
                p[..3].copy_from_slice(&c);
            }
        }
        ClearPixels::Bleed => {
            // Visible pixels seed the flood; each clear pixel takes the colour of the first seed
            // that reaches it (4-neighbour, so the nearest by city-block distance).
            let mut done: Vec<bool> = pixels.iter().map(|p| p[3] > 0).collect();
            let mut queue: VecDeque<usize> = (0..n).filter(|&i| done.get(i) == Some(&true)).collect();
            while let Some(i) = queue.pop_front() {
                let Some(rgb) = pixels.get(i).map(|p| [p[0], p[1], p[2]]) else { continue };
                let (x, y) = (i % w, i / w);
                let around = [(x > 0).then(|| i - 1), (x + 1 < w).then(|| i + 1), (y > 0).then(|| i - w), (y + 1 < h).then(|| i + w)];
                for j in around.into_iter().flatten() {
                    if done.get(j) == Some(&false) {
                        if let Some(flag) = done.get_mut(j) {
                            *flag = true;
                        }
                        if let Some(p) = pixels.get_mut(j) {
                            p[..3].copy_from_slice(&rgb);
                        }
                        queue.push_back(j);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_and_bleed_fill_only_clear_pixels() {
        // 3 × 1: clear, half-visible red, clear
        let src = [0, 0, 0, 0, 255, 0, 0, 128, 0, 0, 0, 0];
        let mut a = src;
        apply(&mut a, 3, 1, ClearPixels::Keep);
        assert_eq!(a, src);
        apply(&mut a, 3, 1, ClearPixels::Color([255, 255, 255]));
        assert_eq!(a, [255, 255, 255, 0, 255, 0, 0, 128, 255, 255, 255, 0]);
        let mut b = src;
        apply(&mut b, 3, 1, ClearPixels::Bleed);
        assert_eq!(b, [255, 0, 0, 0, 255, 0, 0, 128, 255, 0, 0, 0]);
        // nothing visible, or a wrong length: unchanged
        let mut c = [0u8; 8];
        apply(&mut c, 2, 1, ClearPixels::Bleed);
        assert_eq!(c, [0; 8]);
        apply(&mut c, 3, 1, ClearPixels::Color([1, 2, 3]));
        assert_eq!(c, [0; 8]);
    }
}
