#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GeneratedAsset {
    pub(super) path: String,
    pub(super) kind: GeneratedAssetKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum GeneratedAssetKind {
    Png([u8; 4]),
    Bytes(Vec<u8>),
}

impl GeneratedAssetKind {
    pub(super) fn into_bytes(self) -> Result<Vec<u8>, image::ImageError> {
        match self {
            Self::Png(rgba) => {
                let image = image::RgbaImage::from_pixel(1, 1, image::Rgba(rgba));
                let mut encoded = std::io::Cursor::new(Vec::new());
                image::DynamicImage::ImageRgba8(image)
                    .write_to(&mut encoded, image::ImageOutputFormat::Png)?;
                Ok(encoded.into_inner())
            }
            Self::Bytes(bytes) => Ok(bytes),
        }
    }
}
