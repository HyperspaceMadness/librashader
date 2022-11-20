use std::path::Path;

pub struct Image {
    pub bytes: Vec<u8>,
    pub height: u32,
    pub width: u32,
}

impl Image {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, image::ImageError>{
        let image = image::open(path.as_ref())?.to_rgba8();
        let height = image.height();
        let width = image.width();
        
        Ok(Image {
            bytes: image.to_vec(),
            height,
            width
        })
    }
}