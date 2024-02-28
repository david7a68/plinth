#[cfg(target_os = "windows")]
pub mod dx12;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureId(pub(crate) u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SubmitId(pub(crate) u64);
