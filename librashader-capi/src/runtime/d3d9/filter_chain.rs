use crate::ctypes::{
    config_struct, libra_d3d9_filter_chain_t, libra_shader_preset_t, libra_viewport_t, FromUninit,
};
use crate::error::{assert_non_null, assert_some_ptr, LibrashaderError};
use crate::ffi::extern_fn;
use librashader::runtime::d3d9::{FilterChain, FilterChainOptions, FrameOptions};
use std::ffi::c_char;
use std::ffi::CStr;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ops::Deref;
use std::ptr::NonNull;
use std::slice;
use windows::Win32::Graphics::Direct3D9::{IDirect3DDevice9, IDirect3DSurface9, IDirect3DTexture9};

use crate::LIBRASHADER_API_VERSION;
use librashader::runtime::d3d9::error::FilterChainError;
use librashader::runtime::{FilterChainParameters, Size, Viewport};

/// Options for Direct3D 11 filter chain creation.
#[repr(C)]
#[derive(Default, Debug, Clone)]
pub struct filter_chain_d3d9_opt_t {
    /// The librashader API version.
    pub version: LIBRASHADER_API_VERSION,
    /// Whether or not to explicitly disable mipmap
    /// generation regardless of shader preset settings.
    pub force_no_mipmaps: bool,
    /// Disable the shader object cache. Shaders will be
    /// recompiled rather than loaded from the cache.
    pub disable_cache: bool,
}

config_struct! {
    impl FilterChainOptions => filter_chain_d3d9_opt_t {
        0 => [force_no_mipmaps, disable_cache];
    }
}

/// Options for each Direct3D 11 shader frame.
#[repr(C)]
#[derive(Default, Debug, Clone)]
pub struct frame_d3d9_opt_t {
    /// The librashader API version.
    pub version: LIBRASHADER_API_VERSION,
    /// Whether or not to clear the history buffers.
    pub clear_history: bool,
    /// The direction of rendering.
    /// -1 indicates that the frames are played in reverse order.
    pub frame_direction: i32,
    /// The rotation of the output. 0 = 0deg, 1 = 90deg, 2 = 180deg, 3 = 270deg.
    pub rotation: u32,
    /// The total number of subframes ran. Default is 1.
    pub total_subframes: u32,
    /// The current sub frame. Default is 1.
    pub current_subframe: u32,
    /// The expected aspect ratio of the source image.
    ///
    /// This can differ from the actual aspect ratio of the source
    /// image.
    ///
    /// The default is 0, which will automatically
    /// infer the ratio from the source image.
    pub aspect_ratio: f32,
    /// The original frames per second of the source. Default is 1.
    pub frames_per_second: f32,
    /// Time in milliseconds between the current and previous frame. Default is 0.
    pub frametime_delta: u32,
}

config_struct! {
    impl FrameOptions => frame_d3d9_opt_t {
        0 => [clear_history, frame_direction];
        1 => [rotation, total_subframes, current_subframe];
        2 => [aspect_ratio, frames_per_second, frametime_delta];
    }
}

extern_fn! {
    /// Create the filter chain given the shader preset.
    ///
    /// The shader preset is immediately invalidated and must be recreated after
    /// the filter chain is created.
    ///
    /// ## Safety:
    /// - `preset` must be either null, or valid and aligned.
    /// - `options` must be either null, or valid and aligned.
    /// - `device` must not be null.
    /// - `out` must be aligned, but may be null, invalid, or uninitialized.
    fn libra_d3d9_filter_chain_create(
        preset: *mut libra_shader_preset_t,
        device: ManuallyDrop<IDirect3DDevice9>,
        options: *const MaybeUninit<filter_chain_d3d9_opt_t>,
        out: *mut MaybeUninit<libra_d3d9_filter_chain_t>
    ) {
        assert_non_null!(preset);
        let preset = unsafe {
            let preset_ptr = &mut *preset;
            let preset = preset_ptr.take();
            Box::from_raw(preset.unwrap().as_ptr())
        };

        let options = if options.is_null() {
            None
        } else {
            Some(unsafe { options.read() })
        };

        let options = options.map(FromUninit::from_uninit);
        unsafe {
            let chain = FilterChain::load_from_preset(
                *preset,
                &device,
                options.as_ref(),
            )?;

            out.write(MaybeUninit::new(NonNull::new(Box::into_raw(Box::new(
                chain,
            )))))
        }
    }
}

extern_fn! {
    /// Draw a frame with the given parameters for the given filter chain.
    ///
    /// ## Parameters
    /// - `chain` is a handle to the filter chain.
    /// - `frame_count` is the number of frames passed to the shader
    /// - `image` is a pointer to a `IDirect3DTexture9` that will serve as the source image for the frame.
    /// - `out` is a pointer to a `IDirect3DSurface9` that will serve as the render target for the frame.
    ///
    /// - `viewport` is a pointer to a `libra_viewport_t` that specifies the area onto which scissor and viewport
    ///    will be applied to the render target. It may be null, in which case a default viewport spanning the
    ///    entire render target will be used.
    /// - `mvp` is a pointer to an array of 16 `float` values to specify the model view projection matrix to
    ///    be passed to the shader.
    /// - `options` is a pointer to options for the frame. Valid options are dependent on the `LIBRASHADER_API_VERSION`
    ///    passed in. It may be null, in which case default options for the filter chain are used.
    ///
    /// ## Safety
    /// - `chain` may be null, invalid, but not uninitialized. If `chain` is null or invalid, this
    ///    function will return an error.
    /// - `viewport` may be null, or if it is not null, must be an aligned pointer to an instance of `libra_viewport_t`.
    /// - `mvp` may be null, or if it is not null, must be an aligned pointer to 16 consecutive `float`
    ///    values for the model view projection matrix.
    /// - `opt` may be null, or if it is not null, must be an aligned pointer to a valid `frame_d3d9_opt_t`
    ///    struct.
    /// - `out` must not be null.
    /// - `image` must not be null.
    /// - You must ensure that only one thread has access to `chain` before you call this function. Only one
    ///   thread at a time may call this function.
    nopanic fn libra_d3d9_filter_chain_frame(
        chain: *mut libra_d3d9_filter_chain_t,
        frame_count: usize,
        image: ManuallyDrop<IDirect3DTexture9>,
        out: ManuallyDrop<IDirect3DSurface9>,
        viewport: *const libra_viewport_t,
        mvp: *const f32,
        options: *const MaybeUninit<frame_d3d9_opt_t>
    ) mut |chain| {
        assert_some_ptr!(mut chain);

        let mvp = if mvp.is_null() {
            None
        } else {
            Some(<&[f32; 16]>::try_from(unsafe { slice::from_raw_parts(mvp, 16) }).unwrap())
        };

        let options = if options.is_null() {
            None
        } else {
            Some(unsafe { options.read() })
        };

        let viewport = if viewport.is_null() {
            Viewport::new_render_target_sized_origin(out.deref(), mvp)
                .map_err(|e| LibrashaderError::D3D9FilterError(FilterChainError::Direct3DError(e)))?
        } else {
            let viewport = unsafe { viewport.read() };
            Viewport {
                x: viewport.x,
                y: viewport.y,
                output: out.deref(),
                size: Size {
                    height: viewport.height,
                    width: viewport.width
                },
                mvp,
            }
        };

        let options = options.map(FromUninit::from_uninit);


        unsafe {
            chain.frame(image.deref(), &viewport, frame_count, options.as_ref())?;
        }
    }
}

extern_fn! {
    /// Sets a parameter for the filter chain.
    ///
    /// If the parameter does not exist, returns an error.
    /// ## Safety
    /// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_d3d9_filter_chain_t`.
    /// - `param_name` must be either null or a null terminated string.
    fn libra_d3d9_filter_chain_set_param(
        chain: *mut libra_d3d9_filter_chain_t,
        param_name: *const c_char,
        value: f32
    ) |chain| {
        assert_some_ptr!(chain);
        assert_non_null!(param_name);
        unsafe {
            let name = CStr::from_ptr(param_name);
            let name = name.to_str()?;

            if chain.parameters().set_parameter_value(name, value).is_none() {
                return Err(LibrashaderError::UnknownShaderParameter(param_name))
            }
        }
    }
}

extern_fn! {
    /// Gets a parameter for the filter chain.
    ///
    /// If the parameter does not exist, returns an error.
    /// ## Safety
    /// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_d3d9_filter_chain_t`.
    /// - `param_name` must be either null or a null terminated string.
    fn libra_d3d9_filter_chain_get_param(
        chain: *const libra_d3d9_filter_chain_t,
        param_name: *const c_char,
        out: *mut MaybeUninit<f32>
    ) |chain| {
        assert_some_ptr!(chain);
        assert_non_null!(param_name);
        unsafe {
            let name = CStr::from_ptr(param_name);
            let name = name.to_str()?;

            let Some(value) = chain.parameters().parameter_value(name) else {
                return Err(LibrashaderError::UnknownShaderParameter(param_name))
            };

            out.write(MaybeUninit::new(value));
        }
    }
}

extern_fn! {
    /// Sets the number of active passes for this chain.
    ///
    /// ## Safety
    /// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_d3d9_filter_chain_t`.
    fn libra_d3d9_filter_chain_set_active_pass_count(
        chain: *mut libra_d3d9_filter_chain_t,
        value: u32
    ) |chain| {
        assert_some_ptr!(chain);
        chain.parameters().set_passes_enabled(value as usize);
    }
}

extern_fn! {
    /// Gets the number of active passes for this chain.
    ///
    /// ## Safety
    /// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_d3d9_filter_chain_t`.
    fn libra_d3d9_filter_chain_get_active_pass_count(
        chain: *const libra_d3d9_filter_chain_t,
        out: *mut MaybeUninit<u32>
    ) |chain| {
        assert_some_ptr!(chain);
        unsafe {
            let value = chain.parameters().passes_enabled();
            out.write(MaybeUninit::new(value as u32))
        }
    }
}

extern_fn! {
    /// Free a d3d9 filter chain.
    ///
    /// The resulting value in `chain` then becomes null.
    /// ## Safety
    /// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_d3d9_filter_chain_t`.
    fn libra_d3d9_filter_chain_free(chain: *mut libra_d3d9_filter_chain_t) {
        assert_non_null!(chain);
        unsafe {
            let chain_ptr = &mut *chain;
            let chain = chain_ptr.take();
            drop(Box::from_raw(chain.unwrap().as_ptr()))
        };
    }
}
