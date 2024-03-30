use std::{mem::ManuallyDrop, sync::Arc};

use windows::Win32::Graphics::Direct3D12::{
    ID3D12CommandAllocator, ID3D12GraphicsCommandList, ID3D12Resource,
    D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_PLACED_SUBRESOURCE_FOOTPRINT,
    D3D12_RESOURCE_STATE_COPY_DEST, D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
    D3D12_SUBRESOURCE_FOOTPRINT, D3D12_TEXTURE_COPY_LOCATION, D3D12_TEXTURE_COPY_LOCATION_0,
    D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT, D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
    D3D12_TEXTURE_DATA_PITCH_ALIGNMENT, D3D12_TEXTURE_DATA_PLACEMENT_ALIGNMENT,
};

use crate::{
    geometry::{Point, Texel},
    graphics::{
        backend::dx12::{image_barrier, to_dxgi_format},
        PixelBuf,
    },
};

use super::device::Device_;

use super::{SubmitId, TextureId};

pub struct Uploader {
    inner: UploaderImpl,
    device: Arc<Device_>,
}

impl Uploader {
    pub(crate) fn new(device: Arc<Device_>, buffer_size: u64) -> Self {
        Self {
            inner: UploaderImpl::new(&device, buffer_size),
            device,
        }
    }

    pub fn upload_image(&mut self, target: TextureId, pixels: &PixelBuf, origin: Point<Texel>) {
        self.inner
            .upload_image(&self.device, target, pixels, origin);
    }

    // also, no need to have a buffer larger than 64 mib, since that's
    // as large as images are allowed to be. Need to create a new U64
    // limit type.
    //
    // also also, could use an i8 for comparison function, no need for a
    // function pointer.

    pub fn flush_upload_buffer(&mut self) {
        self.inner.flush_upload_buffer(&self.device);
    }
}

pub struct UploaderImpl {
    command_list: ID3D12GraphicsCommandList,
    buffer: ID3D12Resource,
    stages: [UploadBuffer; 1],
    cursor: usize,
    next: *mut u8,
}

impl UploaderImpl {
    pub(crate) fn new(device: &Arc<Device_>, buffer_size: u64) -> Self {
        let buffer = device.create_buffer(buffer_size);

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
        device: &Device_,
        target: TextureId,
        pixels: &PixelBuf,
        origin: Point<Texel>,
    ) {
        let row_size = pixels.row_size(false);
        let row_size_aligned =
            row_size.next_multiple_of(D3D12_TEXTURE_DATA_PITCH_ALIGNMENT as usize);
        debug_assert_eq!(
            row_size_aligned % D3D12_TEXTURE_DATA_PITCH_ALIGNMENT as usize,
            0
        );

        let img_size_aligned = row_size_aligned * usize::try_from(pixels.height().0).unwrap();
        debug_assert_eq!(
            img_size_aligned % D3D12_TEXTURE_DATA_PITCH_ALIGNMENT as usize,
            0
        );

        let pad_rows = row_size != row_size_aligned;

        let buffer_size =
            usize::try_from(unsafe { self.stages[0].stop.offset_from(self.stages[0].base) })
                .unwrap();

        assert!(
                row_size <= buffer_size,
                "Image too large to upload. Increase buffer size to at least the size of a single row of pixels ({} bytes).",
                row_size_aligned
            );

        let copy = |pixels: &PixelBuf, mut buffer: MappedSlice, target: &ID3D12Resource| {
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
                pResource: unsafe { std::mem::transmute_copy(buffer.mem) },
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
                pResource: unsafe { std::mem::transmute_copy(target) },
                Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    SubresourceIndex: 0,
                },
            };

            (src, dst)
        };

        let target = device.get_texture(target);

        if buffer_size >= img_size_aligned {
            if !self.buffer_fits(img_size_aligned as u64) {
                self.flush_upload_buffer(device);
            }

            let buffer = self.buffer_alloc_bytes(device, img_size_aligned);
            let (src, dst) = copy(pixels, buffer, &target);

            image_barrier(
                &self.command_list,
                &target,
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
                &target,
                D3D12_RESOURCE_STATE_COPY_DEST,
                D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
            );
        } else {
            let mut pixels = pixels.clone();
            let mut origin = origin;

            let rows_per_stage = i16::try_from(buffer_size / row_size_aligned).unwrap();

            image_barrier(
                &self.command_list,
                &target,
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
                let (src, dst) = copy(&left, buffer, &target);

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
                &target,
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

    pub fn flush_upload_buffer(&mut self, device: &Device_) {
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

    fn buffer_alloc_bytes(&mut self, device: &Device_, len: usize) -> MappedSlice {
        let buffer = &mut self.stages[self.cursor];

        assert!(unsafe { buffer.stop.offset_from(self.next) } >= isize::try_from(len).unwrap());

        if device.wait(buffer.sync) {
            unsafe { buffer.cmda.Reset() }.unwrap();
            unsafe { self.command_list.Reset(&buffer.cmda, None) }.unwrap();
        }

        let next = unsafe {
            self.next.add(
                self.next
                    .align_offset(D3D12_TEXTURE_DATA_PLACEMENT_ALIGNMENT as usize),
            )
        };
        self.next = unsafe { next.add(len) };

        MappedSlice {
            mem: &self.buffer,
            base: next,
            stop: unsafe { next.add(len) },
            offset: unsafe { next.offset_from(buffer.base) } as u64,
        }
    }

    fn buffer_fits(&self, size: u64) -> bool {
        let size = usize::try_from(size).unwrap();
        let offset = self
            .next
            .align_offset(D3D12_TEXTURE_DATA_PLACEMENT_ALIGNMENT as usize);
        let buffer = &self.stages[self.cursor];
        let capacity = unsafe { buffer.stop.offset_from(self.next) };

        capacity >= isize::try_from(size + offset).unwrap()
    }
}

struct MappedSlice<'a> {
    mem: &'a ID3D12Resource,
    base: *mut u8,
    stop: *mut u8,
    offset: u64,
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
