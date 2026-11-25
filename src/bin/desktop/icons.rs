use eframe::egui::{self, Color32};
use std::io::Cursor;

pub fn desktop_entry() -> &'static str {
    include_str!("../../../assets/desktop/br.ufrgs.hge.MgbOps.desktop")
}

pub fn installation_icons() -> [(u32, &'static [u8]); 5] {
    [
        (32, include_bytes!("../../../assets/branding/hge-32.png")),
        (48, include_bytes!("../../../assets/branding/hge-48.png")),
        (64, include_bytes!("../../../assets/branding/hge-64.png")),
        (128, include_bytes!("../../../assets/branding/hge-128.png")),
        (256, include_bytes!("../../../assets/branding/hge-256.png")),
    ]
}

fn decode(bytes: &[u8]) -> egui::ColorImage {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().expect("embedded PNG header");
    let mut buffer = vec![0; reader.output_buffer_size().expect("embedded PNG size")];
    let info = reader.next_frame(&mut buffer).expect("embedded PNG pixels");
    let rgba = match info.color_type {
        png::ColorType::Rgba => buffer[..info.buffer_size()].to_vec(),
        png::ColorType::Rgb => buffer[..info.buffer_size()]
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
            .collect(),
        _ => panic!("embedded icons must be RGB or RGBA"),
    };
    egui::ColorImage::from_rgba_unmultiplied([info.width as usize, info.height as usize], &rgba)
}

fn image(
    context: &egui::Context,
    name: &'static str,
    bytes: &[u8],
    size: f32,
) -> egui::Image<'static> {
    let id = egui::Id::new(("desktop-icon", name));
    let cached = context.data(|data| data.get_temp::<egui::TextureHandle>(id));
    let texture = cached.unwrap_or_else(|| {
        let texture = context.load_texture(name, decode(bytes), egui::TextureOptions::LINEAR);
        context.data_mut(|data| data.insert_temp(id, texture.clone()));
        texture
    });
    egui::Image::from_texture((texture.id(), egui::Vec2::splat(size)))
        .fit_to_exact_size(egui::Vec2::splat(size))
}

pub fn emblem(context: &egui::Context, size: f32) -> egui::Image<'static> {
    image(
        context,
        "hge",
        include_bytes!("../../../assets/branding/hge-256.png"),
        size,
    )
}

pub fn window_icon() -> egui::IconData {
    let image = decode(include_bytes!("../../../assets/branding/hge-256.png"));
    egui::IconData {
        rgba: image
            .pixels
            .iter()
            .flat_map(Color32::to_srgba_unmultiplied)
            .collect(),
        width: image.size[0] as u32,
        height: image.size[1] as u32,
    }
}

pub fn emoji(context: &egui::Context, glyph: &str, size: f32) -> egui::Image<'static> {
    let (name, bytes) = match glyph {
        "📂" => (
            "folder",
            include_bytes!("../../../assets/icons/folder.png").as_slice(),
        ),
        "🕒" => (
            "recent",
            include_bytes!("../../../assets/icons/recent.png").as_slice(),
        ),
        "⚙" => (
            "settings",
            include_bytes!("../../../assets/icons/settings.png").as_slice(),
        ),
        "🚪" => (
            "close",
            include_bytes!("../../../assets/icons/close.png").as_slice(),
        ),
        "📥" => (
            "import",
            include_bytes!("../../../assets/icons/import.png").as_slice(),
        ),
        "🔄" => (
            "refresh",
            include_bytes!("../../../assets/icons/refresh.png").as_slice(),
        ),
        "🗺" => (
            "map",
            include_bytes!("../../../assets/icons/map.png").as_slice(),
        ),
        "📈" => (
            "charts",
            include_bytes!("../../../assets/icons/charts.png").as_slice(),
        ),
        "📊" => (
            "data",
            include_bytes!("../../../assets/icons/data.png").as_slice(),
        ),
        "📁" => (
            "files",
            include_bytes!("../../../assets/icons/files.png").as_slice(),
        ),
        "💬" => (
            "chat",
            include_bytes!("../../../assets/icons/chat.png").as_slice(),
        ),
        "🔍" => (
            "review",
            include_bytes!("../../../assets/icons/review.png").as_slice(),
        ),
        "📉" => (
            "analysis",
            include_bytes!("../../../assets/icons/analysis.png").as_slice(),
        ),
        "📰" => (
            "bulletin",
            include_bytes!("../../../assets/icons/bulletin.png").as_slice(),
        ),
        "📦" => (
            "bundle",
            include_bytes!("../../../assets/icons/bundle.png").as_slice(),
        ),
        "📋" => (
            "jobs",
            include_bytes!("../../../assets/icons/jobs.png").as_slice(),
        ),
        "🔁" => (
            "reset",
            include_bytes!("../../../assets/icons/reset.png").as_slice(),
        ),
        "▶" => (
            "run",
            include_bytes!("../../../assets/icons/run.png").as_slice(),
        ),
        "⬇" => (
            "acquire",
            include_bytes!("../../../assets/icons/acquire.png").as_slice(),
        ),
        "📐" => (
            "geometry",
            include_bytes!("../../../assets/icons/geometry.png").as_slice(),
        ),
        "🌱" => (
            "soil",
            include_bytes!("../../../assets/icons/soil.png").as_slice(),
        ),
        "🌐" => (
            "language",
            include_bytes!("../../../assets/icons/language.png").as_slice(),
        ),
        "☀" => (
            "light",
            include_bytes!("../../../assets/icons/light.png").as_slice(),
        ),
        "🌙" => (
            "dark",
            include_bytes!("../../../assets/icons/dark.png").as_slice(),
        ),
        _ => panic!("missing desktop emoji asset: {glyph}"),
    };
    image(context, name, bytes, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_brand_and_ribbon_images_decode_in_colour() {
        let icon = window_icon();
        assert_eq!((icon.width, icon.height), (256, 256));
        assert_eq!(icon.rgba.len(), 256 * 256 * 4);
        assert!(icon.rgba.as_chunks::<4>().0.iter().any(|p| p[3] == 0));
        let context = egui::Context::default();
        for glyph in [
            "📂", "🕒", "⚙", "🚪", "📥", "🔄", "🗺", "📈", "📊", "📁", "💬", "🔍", "📉", "📰", "📦",
            "📋", "🔁", "▶", "⬇", "📐", "🌱", "🌐", "☀", "🌙",
        ] {
            let _ = emoji(&context, glyph, 24.);
        }
    }
}
