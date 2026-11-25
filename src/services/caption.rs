//! Bounded, local caption rendering. Font and license are embedded for container builds.
use ab_glyph::{point, Font, FontRef, PxScale, ScaleFont};
use image::{ImageDecoder, Rgba, RgbaImage};
use std::{
    io::{Cursor, Write},
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
pub const MAX_BYTES: usize = 8 * 1024 * 1024;
const PIXELS: u64 = 16_000_000;
const WORK: u64 = 80_000_000;
const DEADLINE: Duration = Duration::from_secs(15);
static JOBS: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(2)));
const FONT: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans-Bold.ttf");
#[allow(dead_code)]
const FONT_LICENSE: &str = include_str!("../../assets/fonts/DejaVu-LICENSE");
type Result<T> = std::result::Result<T, String>;
pub struct Output {
    pub bytes: Vec<u8>,
    pub filename: &'static str,
}
fn err() -> String {
    "Imagem inválida ou limite de processamento excedido (PNG, JPEG ou GIF).".into()
}
fn dimensions(w: u32, h: u32) -> Result<()> {
    if w == 0 || h == 0 || w > 4096 || h > 4096 || u64::from(w) * u64::from(h) > PIXELS {
        Err(err())
    } else {
        Ok(())
    }
}
fn text_valid(s: &str) -> Result<()> {
    if s.trim().is_empty() || s.chars().count() > 300 {
        Err("Texto deve conter de 1 a 300 caracteres.".into())
    } else {
        Ok(())
    }
}
pub fn validate_url(raw: &str) -> Result<reqwest::Url> {
    let u = reqwest::Url::parse(raw).map_err(|_| err())?;
    if u.scheme() != "https"
        || !matches!(
            u.host_str(),
            Some("cdn.discordapp.com" | "media.discordapp.net")
        )
        || u.port().is_some()
        || !u.username().is_empty()
        || u.password().is_some()
        || !u.path().starts_with("/attachments/")
    {
        return Err(err());
    }
    Ok(u)
}
pub fn acquire() -> Result<OwnedSemaphorePermit> {
    JOBS.clone()
        .try_acquire_owned()
        .map_err(|_| "Caption ocupado. Tente novamente em instantes.".into())
}
pub async fn process(
    url: &str,
    size: u32,
    text: String,
    limit: usize,
    permit: OwnedSemaphorePermit,
) -> Result<Output> {
    text_valid(&text)?;
    if size as usize > MAX_BYTES {
        return Err(err());
    }
    let url = validate_url(url)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(DEADLINE)
        .build()
        .map_err(|_| err())?;
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|_| err())?
        .error_for_status()
        .map_err(|_| err())?;
    if response
        .content_length()
        .is_some_and(|n| n > MAX_BYTES as u64)
    {
        return Err(err());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| err())? {
        if chunk.len() > MAX_BYTES - bytes.len() {
            return Err(err());
        }
        bytes.extend_from_slice(&chunk);
    }
    let job = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        render(&bytes, &text, limit)
    });
    tokio::time::timeout(DEADLINE, job)
        .await
        .map_err(|_| err())?
        .map_err(|_| err())?
}
struct Bounded {
    bytes: Vec<u8>,
    limit: usize,
}
impl Write for Bounded {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        if b.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(std::io::Error::other("output limit"));
        }
        self.bytes.extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn band(width: u32, text: &str) -> Result<RgbaImage> {
    let font = FontRef::try_from_slice(FONT).map_err(|_| err())?;
    let size = (width as f32 / 15.0)
        .clamp(8.0, 42.0)
        .min((width as f32 / 3.0).max(1.0));
    let scaled = font.as_scaled(PxScale::from(size));
    let margin = (width / 20).max(1);
    let available = width.saturating_sub(2 * margin) as f32;
    let measure = |s: &str| {
        s.chars()
            .map(|c| scaled.h_advance(font.glyph_id(c)))
            .sum::<f32>()
    };
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() {
            if measure(&format!("{line} {word}")) <= available {
                line.push(' ');
            } else {
                lines.push(std::mem::take(&mut line));
            }
        }
        for c in word.chars() {
            if !line.is_empty() && measure(&format!("{line}{c}")) > available {
                lines.push(std::mem::take(&mut line));
            }
            line.push(c);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    let step = size.ceil() as u32 + 4;
    let height = step * lines.len() as u32 + 2 * margin;
    let mut image = RgbaImage::from_pixel(width, height, Rgba([255; 4]));
    for (n, line) in lines.iter().enumerate() {
        let mut x = (width as f32 - measure(line)) / 2.0;
        let y = margin as f32 + n as f32 * step as f32 + scaled.ascent();
        for c in line.chars() {
            let id = font.glyph_id(c);
            let glyph = id.with_scale_and_position(size, point(x, y));
            if let Some(g) = font.outline_glyph(glyph) {
                let bounds = g.px_bounds();
                g.draw(|gx, gy, v| {
                    let px = gx as i32 + bounds.min.x as i32;
                    let py = gy as i32 + bounds.min.y as i32;
                    if px >= 0 && py >= 0 && px < width as i32 && py < height as i32 {
                        let shade = ((1.0 - v) * 255.0) as u8;
                        image.put_pixel(px as u32, py as u32, Rgba([shade, shade, shade, 255]));
                    }
                });
            }
            x += scaled.h_advance(id);
        }
    }
    Ok(image)
}
fn compose(image: &RgbaImage, band: &RgbaImage) -> RgbaImage {
    let h = ((u64::from(image.height()) * u64::from(band.width())) / u64::from(image.width()))
        .max(1) as u32;
    let resized = image::imageops::resize(
        image,
        band.width(),
        h,
        image::imageops::FilterType::Triangle,
    );
    let mut out = RgbaImage::new(band.width(), h + band.height());
    image::imageops::replace(&mut out, band, 0, 0);
    image::imageops::replace(&mut out, &resized, 0, band.height() as i64);
    out
}
fn apng(bytes: &[u8]) -> Result<bool> {
    let mut p = 8usize;
    while p < bytes.len() {
        let h = bytes.get(p..p + 8).ok_or_else(err)?;
        let n = u32::from_be_bytes(h[..4].try_into().unwrap()) as usize;
        if &h[4..8] == b"acTL" {
            return Ok(true);
        }
        p = p
            .checked_add(n)
            .and_then(|p| p.checked_add(12))
            .ok_or_else(err)?;
        if p > bytes.len() {
            return Err(err());
        }
    }
    Ok(false)
}
pub fn render(bytes: &[u8], text: &str, limit: usize) -> Result<Output> {
    text_valid(text)?;
    if bytes.len() > MAX_BYTES {
        return Err(err());
    }
    let start = Instant::now();
    let format = image::guess_format(bytes).map_err(|_| err())?;
    if format == image::ImageFormat::Gif {
        return render_gif(bytes, text, limit.min(MAX_BYTES), start);
    }
    if !matches!(format, image::ImageFormat::Png | image::ImageFormat::Jpeg)
        || (format == image::ImageFormat::Png && apng(bytes)?)
    {
        return Err(err());
    }
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let decoder = reader.into_decoder().map_err(|_| err())?;
    let (w, h) = decoder.dimensions();
    dimensions(w, h)?;
    let image = image::DynamicImage::from_decoder(decoder)
        .map_err(|_| err())?
        .to_rgba8();
    let band = band(w.min(800), text)?;
    let out = compose(&image, &band);
    if start.elapsed() > DEADLINE {
        return Err(err());
    }
    let mut writer = Bounded {
        bytes: Vec::new(),
        limit: limit.min(MAX_BYTES),
    };
    image::ImageEncoder::write_image(
        image::codecs::png::PngEncoder::new(&mut writer),
        out.as_raw(),
        out.width(),
        out.height(),
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|_| err())?;
    Ok(Output {
        bytes: writer.bytes,
        filename: "caption.png",
    })
}
fn render_gif(bytes: &[u8], text: &str, limit: usize, start: Instant) -> Result<Output> {
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    options.set_memory_limit(gif::MemoryLimit::Bytes(
        (128 * 1024 * 1024).try_into().unwrap(),
    ));
    let mut decoder = options.read_info(Cursor::new(bytes)).map_err(|_| err())?;
    let (w, h) = (decoder.width() as u32, decoder.height() as u32);
    dimensions(w, h)?;
    let band = band(w.min(800), text)?;
    let oh =
        ((u64::from(h) * u64::from(band.width())) / u64::from(w)).max(1) as u32 + band.height();
    let mut canvas = RgbaImage::new(w, h);
    let mut previous = None;
    let mut disposal = gif::DisposalMethod::Any;
    let mut rect = (0, 0, 0, 0);
    let mut writer = Bounded {
        bytes: Vec::new(),
        limit,
    };
    let mut count = 0u64;
    let mut work = 0u64;
    {
        let mut encoder = gif::Encoder::new(&mut writer, band.width() as u16, oh as u16, &[])
            .map_err(|_| err())?;
        encoder.set_repeat(decoder.repeat()).map_err(|_| err())?;
        while let Some(info) = decoder.next_frame_info().map_err(|_| err())? {
            count += 1;
            work += u64::from(w) * u64::from(h) + u64::from(band.width()) * u64::from(oh);
            if count > 150
                || work > WORK
                || start.elapsed() > DEADLINE
                || u32::from(info.left) + u32::from(info.width) > w
                || u32::from(info.top) + u32::from(info.height) > h
            {
                return Err(err());
            }
            let (left, top, fw, fh, delay, next_disposal) = (
                info.left,
                info.top,
                info.width,
                info.height,
                info.delay,
                info.dispose,
            );
            match disposal {
                gif::DisposalMethod::Background => {
                    for y in rect.1..rect.1 + rect.3 {
                        for x in rect.0..rect.0 + rect.2 {
                            canvas.put_pixel(x, y, Rgba([0; 4]));
                        }
                    }
                }
                gif::DisposalMethod::Previous => {
                    if let Some(old) = previous.take() {
                        canvas = old;
                    }
                }
                _ => {}
            }
            if next_disposal == gif::DisposalMethod::Previous {
                previous = Some(canvas.clone());
            }
            let mut buf = vec![0; decoder.buffer_size()];
            decoder.read_into_buffer(&mut buf).map_err(|_| err())?;
            for y in 0..fh as u32 {
                for x in 0..fw as u32 {
                    let p = ((y * fw as u32 + x) * 4) as usize;
                    if buf[p + 3] != 0 {
                        canvas.put_pixel(
                            left as u32 + x,
                            top as u32 + y,
                            Rgba(buf[p..p + 4].try_into().unwrap()),
                        );
                    }
                }
            }
            let mut output = compose(&canvas, &band).into_raw();
            let mut frame =
                gif::Frame::from_rgba_speed(band.width() as u16, oh as u16, &mut output, 10);
            frame.delay = delay;
            frame.dispose = gif::DisposalMethod::Background;
            encoder.write_frame(&frame).map_err(|_| err())?;
            disposal = next_disposal;
            rect = (left as u32, top as u32, fw as u32, fh as u32);
        }
    }
    if count == 0 || start.elapsed() > DEADLINE {
        return Err(err());
    }
    Ok(Output {
        bytes: writer.bytes,
        filename: "caption.gif",
    })
}
