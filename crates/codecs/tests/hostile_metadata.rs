//! Hostile metadata blobs (truncated, bit-flipped, multi-byte text) must never panic the
//! metadata readers/writers or the format sniffer.

use photocraft_codecs::*;

/// A little-endian TIFF/EXIF block with an orientation, resolution and thumbnail IFD.
fn exif_sample() -> Vec<u8> {
    let mut v = b"II*\0".to_vec();
    v.extend_from_slice(&8u32.to_le_bytes());
    let entries: [(u16, u16, u32, u32); 3] = [(0x0112, 3, 1, 6), (0x011A, 5, 1, 38), (0x011B, 5, 1, 46)];
    v.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    for (tag, ty, n, val) in entries {
        v.extend_from_slice(&tag.to_le_bytes());
        v.extend_from_slice(&ty.to_le_bytes());
        v.extend_from_slice(&n.to_le_bytes());
        v.extend_from_slice(&val.to_le_bytes());
    }
    v.extend_from_slice(&0u32.to_le_bytes());
    for _ in 0..4 {
        v.extend_from_slice(&72u32.to_le_bytes());
    }
    v
}

fn exif_variants() -> Vec<Vec<u8>> {
    let base = exif_sample();
    let mut out = vec![Vec::new(), b"II".to_vec(), b"MM\0*".to_vec()];
    for n in 0..=base.len() {
        out.push(base[..n].to_vec());
    }
    for i in 0..base.len() {
        for flip in [0xFFu8, 0x80, 0x01] {
            let mut b = base.clone();
            b[i] ^= flip;
            out.push(b);
        }
    }
    out
}

#[test]
fn exif_readers_and_writers_never_panic() {
    for b in exif_variants() {
        let _ = exif_orientation(&b);
        let _ = upright_exif(&b);
        let _ = exif_resolution(&b);
        let _ = exif_color_space(&b);
        let _ = export_exif(&b, Some((300.0, 300.0)));
        let _ = export_exif(&b, None);
        let _ = export_exif(&b, Some((f32::NAN, f32::INFINITY)));
    }
}

#[test]
fn xmp_readers_and_writers_never_panic() {
    let samples = [
        "",
        "tiff:XResolution=\"",
        "tiff:XResolution='",
        "<tiff:XResolution>",
        "<tiff:XResolution>é",
        "tiff:XResolution=\"é\u{1F600}\" tiff:YResolution=\"3é/1\"",
        "<x:xmpmeta tiff:Orientation=\"6\" tiff:XResolution=\"300/1\" tiff:YResolution=\"300/0\"/>",
        "tiff:Orientation=\"\u{1F600}\"",
        "tiff:Orientation = '6' tiff:Orientation = '8'",
    ];
    for s in samples {
        for (i, _) in s.char_indices().chain(std::iter::once((s.len(), ' '))) {
            let p = &s[..i];
            let _ = xmp_resolution(p);
            let _ = upright_xmp(p);
            let _ = export_xmp(p, Some((72.0, 96.0)));
            let _ = export_xmp(p, Some((f32::NAN, -1.0)));
            let _ = export_xmp(p, None);
        }
    }
}

#[test]
fn photoshop_resources_never_panic() {
    let mut res = b"8BIM\x03\xED\0\0\0\0\0\x10".to_vec();
    res.extend_from_slice(&[0, 72, 0, 0, 0, 1, 0, 1, 0, 72, 0, 0, 0, 1, 0, 1]);
    for n in 0..=res.len() {
        let _ = photoshop_resolution(&res[..n]);
    }
    for i in 0..res.len() {
        let mut b = res.clone();
        b[i] = 0xFF;
        let _ = photoshop_resolution(&b);
    }
}

#[test]
fn sniffer_and_probe_survive_truncated_headers() {
    let heads: [&[u8]; 8] = [
        b"\0\0\0\x18ftypavif\0\0\0\0avifmif1",
        b"\0\0\0\0ftypheic\0\0\0\0",
        b"\xFF\xFF\xFF\xFFftypheic\0\0\0\0mif1",
        b"RIFF\x04\0\0\0WEBPVP8X",
        b"GIF89a\xFF\xFF\xFF\xFF",
        b"BM\0\0\0\0\0\0\0\0\xFF\xFF\xFF\xFF",
        b"qoif\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x03\x00",
        b"#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y 99999999 +X 99999999\n",
    ];
    for h in heads {
        for n in 0..=h.len() {
            let _ = detect(&h[..n]);
            let _ = probe_size(&h[..n]);
            let _ = jpeg_quality(&h[..n]);
            let _ = webp_encoding(&h[..n]);
        }
    }
}
