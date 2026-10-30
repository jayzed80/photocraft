//! Mutated and truncated ICC profiles must be rejected or converted, never panic or hang.

use photocraft_cms::{Builtin, Intent, Profile, Transform};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// Parses `bytes` and, when accepted, pushes a few pixels through both directions.
fn exercise(bytes: &[u8]) {
    let Ok(p) = Profile::parse(bytes) else { return };
    let _ = (p.channels(), p.media_white(), p.is_matrix_shaper(), p.can_be_destination());
    let _ = p.to_bytes();
    let _ = p.gray_as_rgb();
    for intent in [Intent::Perceptual, Intent::RelativeColorimetric, Intent::AbsoluteColorimetric] {
        let _ = (p.supports_intent(intent, true), p.supports_intent(intent, false));
        for bpc in [false, true] {
            for (src, dst) in [(&p, Builtin::Srgb.profile()), (Builtin::Srgb.profile(), &p)] {
                let Ok(t) = Transform::new(src, dst, intent, bpc) else { continue };
                let n = 16;
                let input: Vec<u8> = (0..n * 4).map(|i| (i * 53) as u8).collect();
                let mut out = vec![0u8; n * 4];
                t.convert_u8(&input, n, &mut out, n, false);
                let stride = n;
                let mut f = vec![0.5f32; stride * 2];
                t.apply(&mut f, stride);
            }
        }
    }
}

fn seeds() -> Vec<Vec<u8>> {
    vec![Builtin::Srgb.profile().to_bytes().to_vec(), Builtin::CoatedCmyk.profile().to_bytes().to_vec(), Builtin::LabD50.profile().to_bytes().to_vec()]
}

#[test]
fn truncated_profiles_never_panic() {
    for s in seeds() {
        let step = (s.len() / 400).max(1);
        for len in (0..s.len()).step_by(step) {
            exercise(&s[..len]);
        }
    }
}

#[test]
fn mutated_profiles_never_panic() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for s in seeds() {
        for _ in 0..100 {
            let mut b = s.clone();
            for _ in 0..1 + rng.next() % 6 {
                // Bias towards the header and tag table, where offsets and counts live.
                let span = if rng.next().is_multiple_of(2) { b.len().min(512) } else { b.len() };
                let at = (rng.next() as usize) % span;
                match rng.next() % 3 {
                    0 => b[at] ^= 1 << (rng.next() % 8),
                    1 => b[at] = 0xff,
                    _ => b[at] = 0,
                }
            }
            exercise(&b);
        }
    }
}
