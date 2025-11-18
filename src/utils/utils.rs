use std::sync::Arc;

use gpui::{Image, ImageFormat, ImageSource};
use lofty::picture::Picture;
use symphonia::core::units::Time;

pub fn format_time(time: Time) -> String {
    let sec = time.seconds;
    format!("{:02}:{:02}", sec / 60, sec % 60)
}

/// Convert to image source
pub fn convert_picture(pic: &Picture) -> Option<ImageSource> {
    if let Some(mime) = pic.mime_type() {
        let mtype = match mime {
            lofty::picture::MimeType::Png => Some(ImageFormat::Png),
            lofty::picture::MimeType::Jpeg => Some(ImageFormat::Jpeg),
            lofty::picture::MimeType::Tiff => Some(ImageFormat::Tiff),
            lofty::picture::MimeType::Bmp => Some(ImageFormat::Bmp),
            lofty::picture::MimeType::Gif => Some(ImageFormat::Gif),
            _ => None,
        };
        if mtype != None {
            return Some(ImageSource::Image(Arc::new(Image::from_bytes(
                mtype.unwrap(),
                pic.data().to_vec(),
            ))));
        }
    }
    None
}
