use std::{
    borrow::Cow,
    sync::{
        mpsc::{Sender, SyncSender},
        Arc,
    },
};

use crate::{
    graphics::{Image, PixelBufferRef},
    io,
};

use super::application::Win32Context;

pub enum LoaderMessage {
    AddImageLoader(Box<dyn io::ImageLoader>),
    LoadImage(Cow<'static, str>, SyncSender<Result<Image, io::Error>>),
}

pub fn spawn_resource_thread(
    context: Arc<Win32Context>,
    location: impl io::Location,
) -> Sender<LoaderMessage> {
    let (send, recv) = std::sync::mpsc::channel();
    let device = context.dx12.clone();

    std::thread::spawn(move || {
        let upload_texture = |pixels: &PixelBufferRef| device.upload_texture(pixels);

        let mut image_loaders = Vec::with_capacity(1);

        for message in recv {
            match message {
                LoaderMessage::AddImageLoader(loader) => image_loaders.push(loader),
                LoaderMessage::LoadImage(path, future) => match location.load(&path) {
                    Ok(bytes) => {
                        // iterate in reverse order so that loaders later have
                        // priority and can override earlier loaders
                        for loader in image_loaders.iter_mut().rev() {
                            if loader.can_load(&bytes) {
                                let image = loader.load(&bytes, &mut |pixels| {
                                    let (tex, sub) = upload_texture(pixels);
                                    Image::new(pixels.size, pixels.layout, tex, sub)
                                });

                                future.send(image).unwrap();
                                break;
                            }
                        }
                    }
                    Err(err) => {
                        future.send(Err(err)).unwrap();
                    }
                },
            }
        }
    });

    send
}
