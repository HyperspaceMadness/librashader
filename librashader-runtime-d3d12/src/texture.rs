use crate::descriptor_heap::{CpuStagingHeap, D3D12DescriptorHeapSlot, RenderTargetHeap};
use librashader_common::{FilterMode, ImageFormat, Size, WrapMode};
use std::ops::Deref;
use windows::Win32::Graphics::Direct3D12::{ID3D12Resource, D3D12_CPU_DESCRIPTOR_HANDLE};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT;

#[derive(Clone)]
pub(crate) enum InputDescriptor {
    Owned(D3D12DescriptorHeapSlot<CpuStagingHeap>),
    Raw(D3D12_CPU_DESCRIPTOR_HANDLE),
}

#[derive(Clone)]
pub(crate) enum OutputDescriptor {
    Owned(D3D12DescriptorHeapSlot<RenderTargetHeap>),
    Raw(D3D12_CPU_DESCRIPTOR_HANDLE),
}

impl AsRef<D3D12_CPU_DESCRIPTOR_HANDLE> for InputDescriptor {
    fn as_ref(&self) -> &D3D12_CPU_DESCRIPTOR_HANDLE {
        match self {
            InputDescriptor::Owned(h) => h.deref().as_ref(),
            InputDescriptor::Raw(h) => h,
        }
    }
}

impl AsRef<D3D12_CPU_DESCRIPTOR_HANDLE> for OutputDescriptor {
    fn as_ref(&self) -> &D3D12_CPU_DESCRIPTOR_HANDLE {
        match self {
            OutputDescriptor::Owned(h) => h.deref().as_ref(),
            OutputDescriptor::Raw(h) => h,
        }
    }
}

#[derive(Clone)]
pub struct OutputTexture {
    pub(crate) descriptor: OutputDescriptor,
    pub(crate) size: Size<u32>,
}

impl OutputTexture {
    pub fn new(
        handle: D3D12DescriptorHeapSlot<RenderTargetHeap>,
        size: Size<u32>,
    ) -> OutputTexture {
        let descriptor = OutputDescriptor::Owned(handle);
        OutputTexture { descriptor, size }
    }

    // unsafe since the lifetime of the handle has to survive
    pub unsafe fn new_from_raw(
        handle: D3D12_CPU_DESCRIPTOR_HANDLE,
        size: Size<u32>,
    ) -> OutputTexture {
        let descriptor = OutputDescriptor::Raw(handle);
        OutputTexture { descriptor, size }
    }
}

#[derive(Clone)]
pub struct InputTexture {
    pub(crate) resource: ID3D12Resource,
    pub(crate) descriptor: InputDescriptor,
    pub(crate) size: Size<u32>,
    pub(crate) format: DXGI_FORMAT,
    pub(crate) wrap_mode: WrapMode,
    pub(crate) filter: FilterMode,
}

impl InputTexture {
    pub fn new(
        resource: ID3D12Resource,
        handle: D3D12DescriptorHeapSlot<CpuStagingHeap>,
        size: Size<u32>,
        format: ImageFormat,
        wrap_mode: WrapMode,
        filter: FilterMode,
    ) -> InputTexture {
        let srv = InputDescriptor::Owned(handle);
        InputTexture {
            resource,
            descriptor: srv,
            size,
            format: format.into(),
            wrap_mode,
            filter,
        }
    }

    // unsafe since the lifetime of the handle has to survive
    pub unsafe fn new_from_raw(
        resource: ID3D12Resource,
        handle: D3D12_CPU_DESCRIPTOR_HANDLE,
        size: Size<u32>,
        format: DXGI_FORMAT,
        wrap_mode: WrapMode,
        filter: FilterMode,
    ) -> InputTexture {
        let srv = InputDescriptor::Raw(handle);
        InputTexture {
            resource,
            descriptor: srv,
            size,
            format,
            wrap_mode,
            filter,
        }
    }
}

impl AsRef<InputTexture> for InputTexture {
    fn as_ref(&self) -> &InputTexture {
        self
    }
}
