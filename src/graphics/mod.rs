mod backend;
mod color;
mod image;
mod primitives;

use windows::Win32::Foundation::HWND;

use crate::{
    geometry::{Extent, Point},
    graphics::image::PackedKey,
    system::power::PowerPreference,
    time::{FramesPerSecond, PresentPeriod, PresentTime},
};

use self::backend::{Device, TextureId, Uploader};
pub use self::{
    backend::draw_list::Canvas,
    backend::WindowContext,
    color::Color,
    image::{Error as ImageError, Format, Image, Info as ImageInfo, Layout, PixelBuf},
    primitives::RoundRect,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Backend {
    #[default]
    Auto,
    #[cfg(target_os = "windows")]
    Dx12,
}

#[derive(Debug)]
pub struct GraphicsConfig {
    pub power_preference: PowerPreference,
    pub debug_mode: bool,
    pub backend: Backend,
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        Self {
            power_preference: PowerPreference::MaxPerformance,
            debug_mode: cfg!(debug_assertions),
            backend: Backend::Auto,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameInfo {
    /// The target refresh rate, if a frame rate has been set.
    pub target_frame_rate: Option<FramesPerSecond>,

    pub vblank_period: PresentPeriod,

    /// The estimated time that the next present will occur.
    pub next_present_time: PresentTime,

    /// The time that the last present occurred.
    pub prev_present_time: PresentTime,

    /// The time that the last present was scheduled to occur.
    pub prev_target_present_time: PresentTime,
}

pub(crate) struct Graphics {
    device: Device,
    uploader: Uploader,
}

impl Graphics {
    pub fn new(config: &GraphicsConfig) -> Self {
        let device = Device::new(config);

        let mut uploader = device.create_uploader();

        let white_pixel = device.create_texture(Extent::new(1, 1), Layout::Rgba8, Format::Linear);

        uploader.upload_image(
            white_pixel,
            &PixelBuf::new(
                ImageInfo {
                    extent: Extent::new(1, 1),
                    layout: Layout::Rgba8,
                    format: Format::Linear,
                    stride: 1,
                },
                &[0xFF, 0xFF, 0xFF, 0xFF],
            ),
            Point::new(0, 0),
        );

        Self { device, uploader }
    }

    #[cfg(target_os = "windows")]
    pub fn create_window_context(&self, hwnd: HWND) -> WindowContext {
        self.device.create_context(hwnd)
    }

    /// Creates a new image.
    pub fn create_image(&mut self, info: &ImageInfo) -> Result<Image, ImageError> {
        let texture = self
            .device
            .create_texture(info.extent, info.layout, info.format);

        let image = Image {
            info: info.pack(),
            key: PackedKey::new()
                .with_index(texture.index())
                .with_epoch(texture.epoch()),
        };

        Ok(image)
    }

    /// Uploads pixels for an image.
    ///
    /// The pixel buffer must be the same size as the image.
    pub fn upload_image(&mut self, image: Image, pixels: &PixelBuf) -> Result<(), ImageError> {
        let texture = TextureId::new(image.key.index(), image.key.epoch());

        self.uploader
            .upload_image(texture, pixels, Point::new(0, 0));

        Ok(())
    }

    /// Removes an image from circulation.
    ///
    /// The image may continue to be used in the background until any pending
    /// drawing operations that use this image have completed.
    pub fn remove_image(&mut self, image: Image) {
        let _ = image;
        todo!()
    }

    /// Call to flush staging buffers.
    ///
    /// This does not block.
    pub fn flush_upload_buffer(&mut self) {
        self.uploader.flush_upload_buffer();
    }
}
