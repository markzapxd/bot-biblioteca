use bibliotecaria::services::caption::*;
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use std::io::Cursor;
fn png() -> Vec<u8> {
    let mut b = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(RgbaImage::from_pixel(200, 100, Rgba([255, 0, 0, 255])))
        .write_to(&mut b, ImageFormat::Png)
        .unwrap();
    b.into_inner()
}
#[test]
fn png_has_white_top_black_text_and_original_image() {
    let o = render(&png(), "Olá ação", MAX_BYTES).unwrap();
    let i = image::load_from_memory(&o.bytes).unwrap().to_rgba8();
    assert_eq!(i.width(), 200);
    assert!(i.height() > 100);
    assert_eq!(i.get_pixel(0, 0).0, [255; 4]);
    assert_eq!(i.get_pixel(0, i.height() - 1).0, [255, 0, 0, 255]);
    assert!(i.pixels().any(|p| p.0 == [0, 0, 0, 255]));
}
#[test]
fn rejects_invalid_empty_and_overlong_text() {
    assert!(render(b"bad", "hi", MAX_BYTES).is_err());
    assert!(render(&png(), "  ", MAX_BYTES).is_err());
    assert!(render(&png(), &"a".repeat(301), MAX_BYTES).is_err());
}
#[test]
fn long_unicode_word_wraps() {
    let o = render(&png(), &"á".repeat(120), MAX_BYTES).unwrap();
    assert!(image::load_from_memory(&o.bytes).unwrap().height() > 200);
}
#[test]
fn output_is_bounded() {
    assert!(render(&png(), "hi", 20).is_err());
}
#[test]
fn jpeg_actual_header_supported() {
    let mut b = Cursor::new(Vec::new());
    DynamicImage::new_rgb8(100, 50)
        .write_to(&mut b, ImageFormat::Jpeg)
        .unwrap();
    assert_eq!(
        render(&b.into_inner(), "Oi", MAX_BYTES).unwrap().filename,
        "caption.png"
    );
}
#[test]
fn gif_animation_delays_and_loop_preserved() {
    let mut b = Vec::new();
    {
        let mut e = gif::Encoder::new(&mut b, 80, 40, &[255, 0, 0, 0, 255, 0]).unwrap();
        e.set_repeat(gif::Repeat::Finite(3)).unwrap();
        for (v, d) in [(0, 7), (1, 13)] {
            let mut f = gif::Frame::default();
            f.width = 80;
            f.height = 40;
            f.delay = d;
            f.buffer = vec![v; 3200].into();
            e.write_frame(&f).unwrap();
        }
    }
    let o = render(&b, "Olá", MAX_BYTES).unwrap();
    assert_eq!(o.filename, "caption.gif");
    let mut d = gif::DecodeOptions::new()
        .read_info(Cursor::new(o.bytes))
        .unwrap();
    assert_eq!(d.repeat(), gif::Repeat::Finite(3));
    assert_eq!(d.read_next_frame().unwrap().unwrap().delay, 7);
    assert_eq!(d.read_next_frame().unwrap().unwrap().delay, 13);
    assert!(d.read_next_frame().unwrap().is_none());
}
#[test]
fn oversized_gif_logical_screen_rejected() {
    let mut b = b"GIF89a".to_vec();
    b.extend_from_slice(&[255, 255, 255, 255, 0, 0, 0, 59]);
    assert!(render(&b, "hi", MAX_BYTES).is_err());
}
#[test]
fn transparent_gif_previous_disposal_is_composited() {
    let mut b = Vec::new();
    {
        let mut e = gif::Encoder::new(&mut b, 80, 40, &[255, 0, 0, 0, 255, 0, 0, 0, 255]).unwrap();
        for (color, dispose) in [
            (0, gif::DisposalMethod::Keep),
            (1, gif::DisposalMethod::Previous),
            (2, gif::DisposalMethod::Keep),
        ] {
            let mut f = gif::Frame::default();
            f.width = if color == 0 { 80 } else { 10 };
            f.height = 40;
            f.left = if color == 2 { 20 } else { 0 };
            f.dispose = dispose;
            f.transparent = Some(3);
            f.buffer = vec![color; f.width as usize * 40].into();
            e.write_frame(&f).unwrap();
        }
    }
    let o = render(&b, "Oi", MAX_BYTES).unwrap();
    let mut opt = gif::DecodeOptions::new();
    opt.set_color_output(gif::ColorOutput::RGBA);
    let mut d = opt.read_info(Cursor::new(o.bytes)).unwrap();
    let width = d.width() as usize;
    let height = d.height() as usize;
    d.read_next_frame().unwrap();
    d.read_next_frame().unwrap();
    let f = d.read_next_frame().unwrap().unwrap();
    let at = |x| &f.buffer[((height - 1) * width + x) * 4..((height - 1) * width + x) * 4 + 4];
    assert_eq!(at(0), [255, 0, 0, 255]);
    assert_eq!(at(20), [0, 0, 255, 255]);
}
#[test]
fn frame_count_budget_rejected() {
    let mut b = Vec::new();
    {
        let mut e = gif::Encoder::new(&mut b, 1, 1, &[0, 0, 0, 255, 255, 255]).unwrap();
        let mut f = gif::Frame::default();
        f.width = 1;
        f.height = 1;
        f.buffer = vec![0].into();
        for _ in 0..151 {
            e.write_frame(&f).unwrap();
        }
    }
    assert!(render(&b, "Oi", MAX_BYTES).is_err());
}
#[test]
fn apng_rejected() {
    let mut b = png();
    b.splice(
        33..33,
        [
            0, 0, 0, 8, b'a', b'c', b'T', b'L', 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
    );
    assert!(render(&b, "Oi", MAX_BYTES).is_err());
}
#[test]
fn discord_urls_only() {
    assert!(validate_url("https://cdn.discordapp.com/attachments/1/2/a.png?x=y").is_ok());
    for u in [
        "http://cdn.discordapp.com/attachments/a",
        "https://evil.test/attachments/a",
        "https://cdn.discordapp.com.evil.test/attachments/a",
    ] {
        assert!(validate_url(u).is_err());
    }
}
