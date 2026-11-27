//! The Praetorium emblem shared by the application window and Linux desktop entry.

pub fn rgba(n: u32) -> Vec<u8> {
    let source = include_bytes!("../../../assets/branding/praetorium-512.png");
    let icon = image::load_from_memory_with_format(source, image::ImageFormat::Png)
        .expect("embedded Praetorium icon").into_rgba8();
    image::imageops::resize(&icon, n, n, image::imageops::FilterType::Lanczos3).into_raw()
}
