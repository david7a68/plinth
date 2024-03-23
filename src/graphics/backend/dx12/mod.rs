mod canvas;
mod context;
mod device;
mod shaders;

use std::{mem::ManuallyDrop, sync::Arc};

pub use canvas::Canvas;
pub use context::Context;

use windows::Win32::{
    Foundation::HWND,
    Graphics::{
        Direct3D12::{
            ID3D12CommandAllocator, ID3D12GraphicsCommandList, ID3D12Resource,
            D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_PLACED_SUBRESOURCE_FOOTPRINT,
            D3D12_RESOURCE_BARRIER, D3D12_RESOURCE_BARRIER_0,
            D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES, D3D12_RESOURCE_BARRIER_FLAG_NONE,
            D3D12_RESOURCE_BARRIER_TYPE_TRANSITION, D3D12_RESOURCE_STATES,
            D3D12_RESOURCE_STATE_COPY_DEST, D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
            D3D12_RESOURCE_TRANSITION_BARRIER, D3D12_SUBRESOURCE_FOOTPRINT,
            D3D12_TEXTURE_COPY_LOCATION, D3D12_TEXTURE_COPY_LOCATION_0,
            D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT, D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
            D3D12_TEXTURE_DATA_PITCH_ALIGNMENT,
        },
        DirectComposition::{DCompositionCreateDevice2, IDCompositionDevice},
        Dxgi::Common::{
            DXGI_FORMAT, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_B8G8R8A8_UNORM_SRGB,
            DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_R8G8B8A8_UNORM_SRGB, DXGI_FORMAT_R8_UNORM,
        },
    },
};

use crate::{
    core::static_slot_map::{DefaultKey, Key as _, SlotMap},
    geometry::{Extent, Point, Texel},
    graphics::{
        image::{PackedInfo, PackedKey},
        Format, GraphicsConfig, Image, ImageError, ImageInfo, Layout, PixelBuf,
    },
    limits::IMAGE_EXTENT,
};

use super::SubmitId;

// "Linear subresource copying must be aligned to 512 bytes (with the
// row pitch aligned to D3D12_TEXTURE_DATA_PITCH_ALIGNMENT bytes)." No
// constant, apparently.
//
// from https://learn.microsoft.com/en-us/windows/win32/direct3d12/upload-and-readback-of-texture-data#buffer-alignment
const COPY_ALIGN: usize = 512;

pub struct Graphics {
    device: Arc<device::Device>,
    uploader: Uploader,
    images: SlotMap<1024, ImageResource>,
    compositor: IDCompositionDevice,
}

impl Graphics {
    pub fn new(config: &GraphicsConfig) -> Self {
        let device = Arc::new(device::Device::new(config));

        let white_pixel = {
            let resource = device.alloc_image(Extent::new(1, 1), Layout::Rgba8, Format::Linear);
            let info = PackedInfo::new()
                .with_width(1)
                .with_height(1)
                .with_layout(Layout::Rgba8 as u8)
                .with_format(Format::Linear as u8);

            ImageResource { info, resource }
        };

        let mut uploader = Uploader::new(&device, 1024 * 1024 * 64);

        uploader.upload_image(
            &device,
            &white_pixel.resource,
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

        let images = SlotMap::new(white_pixel);

        let compositor = unsafe { DCompositionCreateDevice2(None) }.unwrap();

        Self {
            device,
            uploader,
            images,
            compositor,
        }
    }

    pub fn create_context(&self, hwnd: HWND) -> Context {
        Context::new(self.device.clone(), &self.compositor, hwnd)
    }

    pub fn create_image(&mut self, info: &ImageInfo) -> Result<Image, ImageError> {
        if !IMAGE_EXTENT.test(info.extent) {
            return Err(ImageError::SizeLimit);
        }

        if !self.images.has_capacity(1) {
            return Err(ImageError::MaxCount);
        }

        let resource = self
            .device
            .alloc_image(info.extent, info.layout, info.format);

        let info = info.packed();

        let key = match self.images.insert(ImageResource { info, resource }) {
            Ok(key) => PackedKey::new()
                .with_index(key.index())
                .with_epoch(key.epoch()),
            Err(_) => panic!("Reserved slot for image was not available!"),
        };

        Ok(Image { info, key })
    }

    pub fn upload_image(&mut self, image: Image, pixels: &PixelBuf) -> Result<(), ImageError> {
        let key = DefaultKey::new(image.key.index(), image.key.epoch());
        let resource = self.images.get_mut(key).unwrap();

        self.uploader
            .upload_image(&self.device, &resource.resource, pixels, Point::new(0, 0));

        Ok(())
    }

    pub fn remove_image(&self, image: Image) {
        let _ = image;
        todo!()
    }

    pub fn upload_flush(&mut self) {
        self.uploader.flush_upload_buffer(&self.device);
    }
}

pub struct Uploader {
    command_list: ID3D12GraphicsCommandList,
    buffer: ID3D12Resource,
    stages: [UploadBuffer; 1],
    cursor: usize,
    next: *mut u8,
}

impl Uploader {
    pub fn new(device: &device::Device, buffer_size: u64) -> Self {
        let buffer = device.alloc_buffer(buffer_size);

        let command_allocator: ID3D12CommandAllocator = unsafe {
            device
                .handle
                .CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT)
        }
        .unwrap();

        let command_list: ID3D12GraphicsCommandList = unsafe {
            device.handle.CreateCommandList(
                0,
                D3D12_COMMAND_LIST_TYPE_DIRECT,
                &command_allocator,
                None,
            )
        }
        .unwrap();

        let stages = [{
            let mut map = std::ptr::null_mut();
            unsafe { buffer.Map(0, None, Some(&mut map)) }.unwrap();
            let map = map.cast();

            UploadBuffer {
                cmda: command_allocator,
                base: map,
                stop: unsafe { map.add(buffer_size as usize) },
                sync: SubmitId(0),
            }
        }];

        let next = stages[0].base;

        Self {
            command_list,
            buffer,
            stages,
            cursor: 0,
            next,
        }
    }

    pub fn upload_image(
        &mut self,
        device: &device::Device,
        target: &ID3D12Resource,
        pixels: &PixelBuf,
        origin: Point<Texel>,
    ) {
        let row_size = pixels.row_size(true);
        let row_size_aligned = row_size + (row_size % D3D12_TEXTURE_DATA_PITCH_ALIGNMENT as usize);
        let img_size_aligned = row_size_aligned * usize::try_from(pixels.height().0).unwrap();
        let pad_rows = row_size != row_size_aligned;

        let buffer_size =
            usize::try_from(unsafe { self.stages[0].stop.offset_from(self.stages[0].base) })
                .unwrap();

        assert!(
                row_size <= buffer_size,
                "Image to large to upload. Increase buffer size to at least the size of a single row of pixels ({} bytes).",
                row_size_aligned
            );

        let copy = |pixels: &PixelBuf, mut buffer: MappedSlice| {
            if pad_rows {
                let mut offset = 0;
                for row in pixels.by_rows() {
                    buffer.copy_from_slice(offset, row.data());
                    offset += row_size_aligned;
                }
            } else {
                buffer.copy_from_slice(0, pixels.data());
            }

            let src = D3D12_TEXTURE_COPY_LOCATION {
                pResource: ManuallyDrop::new(Some(unsafe { ManuallyDrop::take(&mut buffer.mem) })),
                Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
                        Offset: buffer.offset,
                        Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
                            Format: to_dxgi_format(pixels.info().layout, pixels.info().format),
                            Width: pixels.width().0 as u32,
                            Height: pixels.height().0 as u32,
                            Depth: 1,
                            RowPitch: row_size_aligned as u32,
                        },
                    },
                },
            };

            let dst = D3D12_TEXTURE_COPY_LOCATION {
                pResource: ManuallyDrop::new(Some(unsafe { std::mem::transmute_copy(target) })),
                Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    SubresourceIndex: 0,
                },
            };

            (src, dst)
        };

        if self.buffer_fits(img_size_aligned as u64) {
            let buffer = self.buffer_alloc_bytes(device, img_size_aligned);
            let (src, dst) = copy(pixels, buffer);

            image_barrier(
                &self.command_list,
                target,
                D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
                D3D12_RESOURCE_STATE_COPY_DEST,
            );

            unsafe {
                self.command_list.CopyTextureRegion(
                    &dst,
                    origin.x.0 as u32,
                    origin.y.0 as u32,
                    0,
                    &src,
                    None,
                )
            };

            image_barrier(
                &self.command_list,
                target,
                D3D12_RESOURCE_STATE_COPY_DEST,
                D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
            );
        } else if buffer_size >= img_size_aligned {
            self.flush_upload_buffer(device);
            let buffer = self.buffer_alloc_bytes(device, img_size_aligned);
            let (src, dst) = copy(pixels, buffer);

            image_barrier(
                &self.command_list,
                target,
                D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
                D3D12_RESOURCE_STATE_COPY_DEST,
            );

            unsafe {
                self.command_list.CopyTextureRegion(
                    &dst,
                    origin.x.0 as u32,
                    origin.y.0 as u32,
                    0,
                    &src,
                    None,
                )
            };

            image_barrier(
                &self.command_list,
                target,
                D3D12_RESOURCE_STATE_COPY_DEST,
                D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
            );
        } else {
            let mut pixels = pixels.clone();
            let mut origin = origin;

            let rows_per_stage = i16::try_from(buffer_size / row_size_aligned).unwrap();

            image_barrier(
                &self.command_list,
                target,
                D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
                D3D12_RESOURCE_STATE_COPY_DEST,
            );

            while pixels.info().extent.height.0 > 0 {
                let (left, right) = pixels.split_rows(rows_per_stage);

                // For simplicity, flush even if we could have fit part of the
                // image into the current buffer. No-op on an empty buffer.
                //
                // -dz (2024-03-23)
                self.flush_upload_buffer(device);
                let buffer = self.buffer_alloc_bytes(device, buffer_size);
                let (src, dst) = copy(&left, buffer);

                unsafe {
                    self.command_list.CopyTextureRegion(
                        &dst,
                        origin.x.0 as u32,
                        origin.y.0 as u32,
                        0,
                        &src,
                        None,
                    )
                };

                origin.y += rows_per_stage;
                pixels = right;
            }

            image_barrier(
                &self.command_list,
                target,
                D3D12_RESOURCE_STATE_COPY_DEST,
                D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
            );

            debug_assert_eq!(origin.y, pixels.height());
        }
    }

    // also, no need to have a buffer larger than 64 mib, since that's
    // as large as images are allowed to be. Need to create a new U64
    // limit type.
    //
    // also also, could use an i8 for comparison function, no need for a
    // function pointer.

    pub fn flush_upload_buffer(&mut self, device: &device::Device) {
        let buffer = &mut self.stages[self.cursor];
        if self.next.cast_const() == buffer.base {
            return;
        }

        let buffer = &mut self.stages[self.cursor];

        unsafe { self.command_list.Close() }.unwrap();
        buffer.sync = device.submit(&self.command_list);

        self.cursor = (self.cursor + 1) % self.stages.len();
        self.next = self.stages[self.cursor].base;
    }

    fn buffer_alloc_bytes(&mut self, device: &device::Device, len: usize) -> MappedSlice {
        let buffer = &mut self.stages[self.cursor];

        assert!(unsafe { buffer.stop.offset_from(self.next) } >= isize::try_from(len).unwrap());

        if device.wait(buffer.sync) {
            unsafe { buffer.cmda.Reset() }.unwrap();
            unsafe { self.command_list.Reset(&buffer.cmda, None) }.unwrap();
        }

        let next = unsafe { self.next.add(self.next.align_offset(COPY_ALIGN)) };
        self.next = unsafe { next.add(len) };

        MappedSlice {
            mem: ManuallyDrop::new(unsafe { std::mem::transmute_copy(&self.buffer) }),
            base: next,
            stop: unsafe { next.add(len) },
            offset: unsafe { next.offset_from(buffer.base) } as u64,
            _marker: std::marker::PhantomData,
        }
    }

    fn buffer_fits(&self, size: u64) -> bool {
        let size = usize::try_from(size).unwrap();
        let offset = self.next.align_offset(COPY_ALIGN);
        let buffer = &self.stages[self.cursor];
        let capacity = unsafe { buffer.stop.offset_from(self.next) };

        capacity >= isize::try_from(size + offset).unwrap()
    }
}

struct MappedSlice<'a> {
    mem: ManuallyDrop<ID3D12Resource>,
    base: *mut u8,
    stop: *mut u8,
    offset: u64,
    _marker: std::marker::PhantomData<&'a mut [u8]>,
}

impl MappedSlice<'_> {
    fn copy_from_slice(&mut self, start: usize, slice: &[u8]) {
        let size = unsafe { self.stop.offset_from(self.base) as usize };
        assert!(start + slice.len() <= size);

        let len = slice.len();
        unsafe { self.base.copy_from_nonoverlapping(slice.as_ptr(), len) };
    }
}

pub struct UploadBuffer {
    cmda: ID3D12CommandAllocator,
    base: *mut u8,
    stop: *mut u8,
    sync: SubmitId,
}

struct ImageResource {
    info: PackedInfo,
    resource: ID3D12Resource,
}

pub fn image_barrier(
    command_list: &ID3D12GraphicsCommandList,
    image: &ID3D12Resource,
    from: D3D12_RESOURCE_STATES,
    to: D3D12_RESOURCE_STATES,
) {
    let transition = D3D12_RESOURCE_TRANSITION_BARRIER {
        pResource: unsafe { std::mem::transmute_copy(image) },
        Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
        StateBefore: from,
        StateAfter: to,
    };

    let barrier = D3D12_RESOURCE_BARRIER {
        Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
        Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
        Anonymous: D3D12_RESOURCE_BARRIER_0 {
            Transition: ManuallyDrop::new(transition),
        },
    };

    unsafe { command_list.ResourceBarrier(&[barrier]) };
}

fn to_dxgi_format(layout: Layout, format: Format) -> DXGI_FORMAT {
    match (layout, format) {
        (_, Format::Unkown) => panic!("Unknown format"),
        (Layout::Rgba8, Format::Srgb) => DXGI_FORMAT_R8G8B8A8_UNORM_SRGB,
        (Layout::Rgba8, Format::Linear) => DXGI_FORMAT_R8G8B8A8_UNORM,
        (Layout::Rgba8Vector, Format::Srgb) => DXGI_FORMAT_R8G8B8A8_UNORM_SRGB,
        (Layout::Rgba8Vector, Format::Linear) => DXGI_FORMAT_R8G8B8A8_UNORM,
        (Layout::Bgra8, Format::Srgb) => DXGI_FORMAT_B8G8R8A8_UNORM_SRGB,
        (Layout::Bgra8, Format::Linear) => DXGI_FORMAT_B8G8R8A8_UNORM,
        (Layout::Alpha8, Format::Linear) => DXGI_FORMAT_R8_UNORM,
        (Layout::Alpha8, Format::Srgb) => panic!("Alpha8 is not supported in SRGB format"),
    }
}
