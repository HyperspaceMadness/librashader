# librashader

![image](https://user-images.githubusercontent.com/1000503/202991618-e3e38e05-f0de-429d-a3ee-4cd0b077f88f.png)

<small>*crt-royale-fake-bloom*</small>

A preprocessor, compiler, and runtime for RetroArch 'slang' shaders, rewritten in pure Rust.

Heavily WIP.

## Supported Render APIs
librashader supports OpenGL 3, Vulkan, DirectX 11, and DirectX 12. Support is WIP for all runtimes except OpenGL 3. Older versions
of DirectX and OpenGL, as well as Metal, are not supported (but pull-requests are welcome).

| **API**    | **Status** | **`librashader` feature** |
|------------|------------|---------------------------|
| OpenGL 3   | ✔          | `gl`                      |
| Vulkan     | 🚧         | `vk`                      |
| DirectX 11 | 🚧         | `dx11`                    |
| DirectX 12 | 🚧         | `dx12`                    |
| OpenGL 2   | ❌          |                           |
| DirectX 9  | ❌          |                           |
| Metal      | ❌          |                           |


## License
The core parts of librashader such as the preprocessor, the preset parser, 
the reflection library, and the runtimes, are all licensed under the Mozilla Public License version 2.0.

The librashader C API, i.e. its headers and definitions, *not its implementation in `librashader_capi`*,
are unique to librashader and are more permissively licensed, and may allow you to use librashader in your permissively 
licensed or proprietary project.

While the code for `librashader_capi` (`librashader.so` and `rashader.dll`) is still under MPL-2.0, 
you may use librashader in proprietary works by linking against the MIT licensed `librashader_ld`, 
which implements the librashader C API, and thunks its calls to any `librashader.so` or `rashader.dll` 
library found in the load path, *provided that `librashader.so` or `rashader.dll` are distributed under the restrictions
of MPLv2*. 

Note that if your project is not compatible with MPLv2, you **can not distribute `librashader.so` or `rashader.dll`**
alongside your project, **only `librashader-ld.so` or `rashader-ld.dll`**, which will do nothing without a librashader
implementation in the load path. The end user must obtain the implementation of librashader themselves.

You may, at your discretion, choose to distribute `librashader` under the terms of MPL-2.0 instead of GPLv3. 