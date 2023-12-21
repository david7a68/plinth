use windows::Win32::Graphics::Direct3D12::{ID3D12Resource, D3D12_CPU_DESCRIPTOR_HANDLE};

use crate::graphics::backend::{Image, ImageImpl};

pub struct Dx12Image {
    pub handle: ID3D12Resource,
    pub render_target_view: D3D12_CPU_DESCRIPTOR_HANDLE,
}

impl<'a> TryFrom<&'a Image> for &'a Dx12Image {
    type Error = ();

    fn try_from(wrapper: &'a Image) -> Result<Self, Self::Error> {
        match &wrapper.image {
            ImageImpl::Dx12(image) => Ok(image),
        }
    }
}
