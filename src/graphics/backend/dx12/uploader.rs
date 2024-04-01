use windows::{
    core::Interface,
    Win32::Graphics::Direct3D12::{
        ID3D12CommandAllocator, ID3D12Device, ID3D12GraphicsCommandList, ID3D12Resource,
        D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_PLACED_SUBRESOURCE_FOOTPRINT,
        D3D12_RESOURCE_STATE_COPY_DEST, D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
        D3D12_SUBRESOURCE_FOOTPRINT, D3D12_TEXTURE_COPY_LOCATION, D3D12_TEXTURE_COPY_LOCATION_0,
        D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT, D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
        D3D12_TEXTURE_DATA_PITCH_ALIGNMENT, D3D12_TEXTURE_DATA_PLACEMENT_ALIGNMENT,
    },
};

use crate::{
    geometry::{Point, Texel},
    graphics::{
        backend::dx12::{image_barrier, to_dxgi_format},
        RasterBuf,
    },
};

use super::device::{alloc_upload_buffer, Queue};

use super::SubmitId;

pub struct Uploader {
    command_list: ID3D12GraphicsCommandList,
    stages: [UploadBuffer; 1],
    cursor: usize,
}

impl Uploader {
    pub(crate) fn new(device: &ID3D12Device, buffer_size: u64) -> Self {
        let command_allocator: ID3D12CommandAllocator =
            unsafe { device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT) }.unwrap();

        let command_list: ID3D12GraphicsCommandList = unsafe {
            device.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &command_allocator, None)
        }
        .unwrap();

        let stages = [{
            let buffer = alloc_upload_buffer(device, buffer_size);

            let mut map = std::ptr::null_mut();
            unsafe { buffer.Map(0, None, Some(&mut map)) }.unwrap();
            let map = map.cast();

            UploadBuffer {
                data: buffer,
                cmda: command_allocator,
                base: map,
                next: map,
                stop: unsafe { map.add(buffer_size as usize) },
                sync: None,
                _pad: [0; 2],
            }
        }];

        Self {
            command_list,
            stages,
            cursor: 0,
        }
    }

    pub fn upload_image(
        &mut self,
        queue: &Queue,
        target: &ID3D12Resource,
        pixels: &RasterBuf,
        origin: Point<Texel>,
    ) {
        let row_alignment = D3D12_TEXTURE_DATA_PITCH_ALIGNMENT as usize;

        let row_size = pixels.row_size();
        let row_size_aligned = row_size.next_multiple_of(row_alignment);
        let img_size_aligned = row_size_aligned * usize::try_from(pixels.height().0).unwrap();

        debug_assert_eq!(row_size_aligned % row_alignment, 0);
        debug_assert_eq!(img_size_aligned % row_alignment, 0);

        let buffer_size = self.stages[0].size();

        assert!(
                row_size <= buffer_size,
                "Image too large to upload. Increase buffer size to at least the size of a single row of pixels ({} bytes).",
                row_size_aligned
            );

        {
            let buffer = &mut self.stages[self.cursor];
            if let Some(sync) = buffer.sync.take() {
                queue.wait(sync);

                unsafe { buffer.cmda.Reset() }.unwrap();
                unsafe { self.command_list.Reset(&buffer.cmda, None) }.unwrap();
            }
        }

        image_barrier(
            &self.command_list,
            target,
            D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
            D3D12_RESOURCE_STATE_COPY_DEST,
        );

        let rows_per_chunk = buffer_size / row_size_aligned;
        let bytes_per_chunk = rows_per_chunk * row_size;

        let mut advancing_y = origin.y.0 as usize;

        let fmt = to_dxgi_format(pixels.info().layout, pixels.info().format);

        let dst = D3D12_TEXTURE_COPY_LOCATION {
            pResource: unsafe { std::mem::transmute_copy(target) },
            Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
            Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                SubresourceIndex: 0,
            },
        };

        for chunk in pixels.data().chunks(bytes_per_chunk) {
            let height = chunk.len() / row_size;

            let mut buffer = self.buffer_alloc_bytes(queue, height * row_size_aligned);

            if row_size == row_size_aligned {
                buffer.copy_from_slice(0, chunk);
            } else {
                let mut offset = 0;
                for row in chunk.chunks(row_size) {
                    buffer.copy_from_slice(offset, row);
                    offset += row_size_aligned;
                }
            }

            let src = D3D12_TEXTURE_COPY_LOCATION {
                pResource: unsafe { std::mem::transmute_copy(buffer.mem) },
                Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
                        Offset: buffer.offset,
                        Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
                            Format: fmt,
                            Width: pixels.width().0 as u32,
                            Height: height as u32,
                            Depth: 1,
                            RowPitch: row_size_aligned as u32,
                        },
                    },
                },
            };

            unsafe {
                self.command_list.CopyTextureRegion(
                    &dst,
                    origin.x.0 as u32,
                    advancing_y as u32,
                    0,
                    &src,
                    None,
                )
            };

            advancing_y += rows_per_chunk;
        }

        image_barrier(
            &self.command_list,
            target,
            D3D12_RESOURCE_STATE_COPY_DEST,
            D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
        );
    }

    // also, no need to have a buffer larger than 64 mib, since that's
    // as large as images are allowed to be. Need to create a new U64
    // limit type.
    //
    // also also, could use an i8 for comparison function, no need for a
    // function pointer.

    pub fn flush_upload_buffer(&mut self, queue: &Queue) {
        let n_stages = self.stages.len();

        let buffer = &mut self.stages[self.cursor];

        if buffer.used() == 0 {
            return;
        }

        unsafe { self.command_list.Close() }.unwrap();
        buffer.sync = Some(queue.submit(&self.command_list.cast().unwrap()));

        self.cursor = (self.cursor + 1) % n_stages; // use var n_stages to satisfy the borrow checker
        buffer.next = buffer.base;

        debug_assert_eq!(buffer.used(), 0);
    }

    fn buffer_alloc_bytes(&mut self, queue: &Queue, len: usize) -> MappedSlice {
        if !self.stages[self.cursor].has_capacity(len) {
            self.flush_upload_buffer(queue);
        }

        let buffer = &mut self.stages[self.cursor];

        if let Some(sync) = buffer.sync.take() {
            queue.wait(sync);

            unsafe { buffer.cmda.Reset() }.unwrap();
            unsafe { self.command_list.Reset(&buffer.cmda, None) }.unwrap();
        }

        buffer.alloc_bytes(len).expect("Buffer too small")
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

struct UploadBuffer {
    data: ID3D12Resource,
    cmda: ID3D12CommandAllocator,
    base: *mut u8,
    next: *mut u8,
    stop: *mut u8,
    sync: Option<SubmitId>,

    #[allow(dead_code)]
    _pad: [u64; 2],
}

impl UploadBuffer {
    pub fn size(&self) -> usize {
        (unsafe { self.stop.offset_from(self.base) } as usize)
    }

    pub fn used(&self) -> usize {
        (unsafe { self.next.offset_from(self.base) } as usize)
    }

    pub fn has_capacity(&self, len: usize) -> bool {
        let alignment = self
            .next
            .align_offset(D3D12_TEXTURE_DATA_PLACEMENT_ALIGNMENT as usize);

        let size = len + alignment;

        usize::try_from(unsafe { self.stop.offset_from(self.next) }).unwrap() >= size
    }

    pub fn alloc_bytes(&mut self, len: usize) -> Option<MappedSlice> {
        let alignment = self
            .next
            .align_offset(D3D12_TEXTURE_DATA_PLACEMENT_ALIGNMENT as usize);

        let size = len + alignment;

        if (unsafe { self.stop.offset_from(self.next) } as usize) < size {
            return None;
        }

        let next = unsafe { self.next.add(alignment) };
        self.next = unsafe { next.add(len) };

        Some(MappedSlice {
            mem: &self.data,
            base: next,
            stop: unsafe { next.add(len) },
            offset: unsafe { next.offset_from(self.base) } as u64,
        })
    }
}
