use crate::filter_chain::FilterCommon;
use crate::texture::InputTexture;
use librashader_common::{ImageFormat, Size, Viewport};
use librashader_preprocess::ShaderSource;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::reflect::semantics::{
    BindingStage, MemberOffset, TextureBinding, UniformBinding,
};
use librashader_reflect::reflect::ShaderReflection;
use rustc_hash::FxHashMap;

use librashader_runtime::binding::{BindSemantics, TextureInput};
use librashader_runtime::quad::QuadType;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Buffer, ID3D11InputLayout, ID3D11PixelShader, ID3D11SamplerState,
    ID3D11ShaderResourceView, ID3D11VertexShader, D3D11_MAPPED_SUBRESOURCE,
    D3D11_MAP_WRITE_DISCARD,
};

use crate::render_target::RenderTarget;
use crate::samplers::SamplerSet;
use crate::{error, D3D11OutputView};
use librashader_runtime::uniforms::{UniformStorage, UniformStorageAccess};

pub struct ConstantBufferBinding {
    pub binding: u32,
    pub size: u32,
    pub stage_mask: BindingStage,
    pub buffer: ID3D11Buffer,
}

// slang_process.cpp 141
pub struct FilterPass {
    pub reflection: ShaderReflection,
    pub vertex_shader: ID3D11VertexShader,
    pub vertex_layout: ID3D11InputLayout,
    pub pixel_shader: ID3D11PixelShader,

    pub uniform_bindings: FxHashMap<UniformBinding, MemberOffset>,

    pub uniform_storage: UniformStorage,
    pub uniform_buffer: Option<ConstantBufferBinding>,
    pub push_buffer: Option<ConstantBufferBinding>,
    pub source: ShaderSource,
    pub config: ShaderPassConfig,
}

// https://doc.rust-lang.org/nightly/core/array/fn.from_fn.html is not ~const :(
const NULL_TEXTURES: &[Option<ID3D11ShaderResourceView>; 16] = &[
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
];

impl TextureInput for InputTexture {
    fn size(&self) -> Size<u32> {
        self.view.size
    }
}

impl BindSemantics for FilterPass {
    type InputTexture = InputTexture;
    type SamplerSet = SamplerSet;
    type DescriptorSet<'a> = (
        &'a mut [Option<ID3D11ShaderResourceView>; 16],
        &'a mut [Option<ID3D11SamplerState>; 16],
    );
    type DeviceContext = ();
    type UniformOffset = MemberOffset;

    fn bind_texture<'a>(
        descriptors: &mut Self::DescriptorSet<'a>,
        samplers: &Self::SamplerSet,
        binding: &TextureBinding,
        texture: &Self::InputTexture,
        _device: &Self::DeviceContext,
    ) {
        let (texture_binding, sampler_binding) = descriptors;
        texture_binding[binding.binding as usize] = Some(texture.view.handle.clone());
        sampler_binding[binding.binding as usize] =
            Some(samplers.get(texture.wrap_mode, texture.filter).clone());
    }
}

// slang_process.cpp 229
impl FilterPass {
    pub fn get_format(&self) -> ImageFormat {
        let fb_format = self.source.format;
        if let Some(format) = self.config.get_format_override() {
            format
        } else if fb_format == ImageFormat::Unknown {
            ImageFormat::R8G8B8A8Unorm
        } else {
            fb_format
        }
    }

    // framecount should be pre-modded
    fn build_semantics<'a>(
        &mut self,
        pass_index: usize,
        parent: &FilterCommon,
        mvp: &[f32; 16],
        frame_count: u32,
        frame_direction: i32,
        fb_size: Size<u32>,
        viewport_size: Size<u32>,
        mut descriptors: (
            &'a mut [Option<ID3D11ShaderResourceView>; 16],
            &'a mut [Option<ID3D11SamplerState>; 16],
        ),
        original: &InputTexture,
        source: &InputTexture,
    ) {
        Self::bind_semantics(
            &(),
            &parent.samplers,
            &mut self.uniform_storage,
            &mut descriptors,
            mvp,
            frame_count,
            frame_direction,
            fb_size,
            viewport_size,
            original,
            source,
            &self.uniform_bindings,
            &self.reflection.meta.texture_meta,
            parent.output_textures[0..pass_index]
                .iter()
                .map(|o| o.as_ref()),
            parent.feedback_textures.iter().map(|o| o.as_ref()),
            parent.history_textures.iter().map(|o| o.as_ref()),
            parent.luts.iter().map(|(u, i)| (*u, i.as_ref())),
            &self.source.parameters,
            &parent.config.parameters,
        );
    }

    pub(crate) fn draw(
        &mut self,
        pass_index: usize,
        parent: &FilterCommon,
        frame_count: u32,
        frame_direction: i32,
        viewport: &Viewport<D3D11OutputView>,
        original: &InputTexture,
        source: &InputTexture,
        output: RenderTarget,
        vbo_type: QuadType,
    ) -> error::Result<()> {
        let _device = &parent.d3d11.device;
        let context = &parent.d3d11.current_context;

        if self.config.mipmap_input && !parent.disable_mipmaps {
            unsafe {
                context.GenerateMips(&source.view.handle);
                // context.GenerateMips(&original.view.handle);
            }
        }
        unsafe {
            context.IASetInputLayout(&self.vertex_layout);
            context.VSSetShader(&self.vertex_shader, None);
            context.PSSetShader(&self.pixel_shader, None);
        }

        let mut textures: [Option<ID3D11ShaderResourceView>; 16] = std::array::from_fn(|_| None);
        let mut samplers: [Option<ID3D11SamplerState>; 16] = std::array::from_fn(|_| None);
        let descriptors = (&mut textures, &mut samplers);

        self.build_semantics(
            pass_index,
            parent,
            output.mvp,
            frame_count,
            frame_direction,
            output.output.size,
            viewport.output.size,
            descriptors,
            original,
            source,
        );

        if let Some(ubo) = &self.uniform_buffer {
            // upload uniforms
            unsafe {
                let mut map = D3D11_MAPPED_SUBRESOURCE::default();
                context.Map(&ubo.buffer, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut map))?;
                std::ptr::copy_nonoverlapping(
                    self.uniform_storage.ubo_pointer(),
                    map.pData.cast(),
                    ubo.size as usize,
                );
                context.Unmap(&ubo.buffer, 0);
            }

            if ubo.stage_mask.contains(BindingStage::VERTEX) {
                unsafe { context.VSSetConstantBuffers(ubo.binding, Some(&[ubo.buffer.clone()])) }
            }
            if ubo.stage_mask.contains(BindingStage::FRAGMENT) {
                unsafe { context.PSSetConstantBuffers(ubo.binding, Some(&[ubo.buffer.clone()])) }
            }
        }

        if let Some(push) = &self.push_buffer {
            // upload push constants
            unsafe {
                let mut map = D3D11_MAPPED_SUBRESOURCE::default();
                context.Map(&push.buffer, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut map))?;
                std::ptr::copy_nonoverlapping(
                    self.uniform_storage.push_pointer(),
                    map.pData.cast(),
                    push.size as usize,
                );
                context.Unmap(&push.buffer, 0);
            }

            if push.stage_mask.contains(BindingStage::VERTEX) {
                unsafe { context.VSSetConstantBuffers(push.binding, Some(&[push.buffer.clone()])) }
            }
            if push.stage_mask.contains(BindingStage::FRAGMENT) {
                unsafe { context.PSSetConstantBuffers(push.binding, Some(&[push.buffer.clone()])) }
            }
        }

        unsafe {
            // reset RTVs
            context.OMSetRenderTargets(None, None);
        }

        unsafe {
            // SAFETY: Niche optimization for Option<NonNull<T>>
            // Assumes that IUnknown is defined as IUnknown(std::ptr::NonNull<std::ffi::c_void>)
            const _: () = assert!(
                std::mem::size_of::<Option<windows::core::IUnknown>>()
                    == std::mem::size_of::<windows::core::IUnknown>()
            );
            context.PSSetShaderResources(0, Some(std::mem::transmute(textures.as_ref())));
            context.PSSetSamplers(0, Some(std::mem::transmute(samplers.as_ref())));

            context.OMSetRenderTargets(Some(&[output.output.rtv.clone()]), None);
            context.RSSetViewports(Some(&[output.output.viewport]))
        }

        parent.draw_quad.draw_quad(context, vbo_type);

        unsafe {
            // unbind resources.
            context.PSSetShaderResources(0, Some(std::mem::transmute(NULL_TEXTURES.as_ref())));
            context.OMSetRenderTargets(None, None);
        }
        Ok(())
    }
}
