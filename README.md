# rend3

![GitHub Workflow Status](https://img.shields.io/github/workflow/status/BVE-Reborn/rend3/CI)
[![Crates.io](https://img.shields.io/crates/v/rend3)](https://crates.io/crates/rend3)
[![Documentation](https://docs.rs/rend3/badge.svg)](https://docs.rs/rend3)
![License](https://img.shields.io/crates/l/rend3)
[![Matrix](https://img.shields.io/static/v1?label=rend3%20dev&message=%23rend3&color=blueviolet&logo=matrix)](https://matrix.to/#/#rend3:matrix.org)
[![Matrix](https://img.shields.io/static/v1?label=rend3%20users&message=%23rend3-users&color=blueviolet&logo=matrix)](https://matrix.to/#/#rend3-users:matrix.org)
[![Discord](https://img.shields.io/discord/451037457475960852?color=7289DA&label=discord)](https://discord.gg/mjxXTVzaDg)


Easy to use, customizable, efficient 3D renderer library built on wgpu.

Library is under active development. While internals are might change in the
future, the external api remains stable, with only minor changes occuring as
features are added.

# Examples

Take a look at the [examples] for getting started with the api. The examples
will show how the core library and helper crates can be used.

[examples]: https://github.com/BVE-Reborn/rend3/tree/trunk/examples

#### Screenshots

These screenshots are from the scene-viewer example.

![scifi-base](https://raw.githubusercontent.com/BVE-Reborn/rend3/trunk/examples/scene-viewer/scifi-base.jpg)
![example](https://raw.githubusercontent.com/BVE-Reborn/rend3/trunk/examples/scene-viewer/screenshot.jpg)
![bistro](https://raw.githubusercontent.com/BVE-Reborn/rend3/trunk/examples/scene-viewer/bistro.jpg)
![emerald-square](https://raw.githubusercontent.com/BVE-Reborn/rend3/trunk/examples/scene-viewer/emerald-square.jpg)

# Crates

The `rend3` ecosystem is composed of a couple core crates which provide most
of the functionality and exensibility to the library, extension crates, and
integration crates

#### Core

- `rend3`: The core crate. Performs all handling of world data, provides the
  Renderer and RenderGraph and defines vocabulary types.
- `rend3-routine`: Implementation of various "Render Routines" on top of the
  RenderGraph. Also provides for re-usable graphics work. Provides PBR
  rendering, Skyboxes, Shadow Rendering, and Tonemapping.

#### Extensions

There are extension crates that are not required, but provide pre-made bits
of useful code that I would recommend using.

- `rend3-framework`: Vastly simplifies correct handling of the window and
  surface across platforms.
- `rend3-gltf`: Modular gltf file and scene loader.

#### Integration

Integration with other external libraries are also offered. Due to external
dependencies, the versions of these may increase at a much higher rate than
the rest of the ecosystem.

- `rend3-egui`: Integration with the [egui](https://github.com/emilk/egui)
  immediate mode gui.
- `rend3-imgui`: Integration with the [imgui](https://github.com/ocornut/imgui)
  immediate mode gui.

# Features and Platform Support

rend3 supports two different rendering profiles one for speed and one for
compatibility.

#### Profile Features

The modern profile not only offloads a lot more work to the gpu, it can do more
aggressive performance optimizations.

| Profile  | Texture Access | Object Culling | Triangle Culling | Draw Calls          |
|:---------|----------------|----------------|------------------|---------------------|
| Modern   | Bindless       | On GPU         | On GPU           | Merged Indirect     |
| Legacy   | Bound          | On CPU         | ❌                | Instanced Direct    |

#### Profile Support

The following table shows support of various profiles on various apis/platforms. This will
hopefully help you judge what your target demographic supports.

| OS                   | API    | GPU                                        | Modern | Legacy |
|----------------------|--------|--------------------------------------------|:------:|:------:|
| Windows 7+           | Vulkan | AMD / NVIDIA                               | ✅    | —       |
|                      | Vulkan | Intel 6XXX+                                | ❌      | ✅     |
|                      | Dx11   | Intel 2XXX+                                | ❌      | 🚧     |
| Windows 10+          | Dx12   | Intel 6XXX+ / AMD GCN 2+ / NVIDIA 6XX+     | 🚧    | ✅     |
| MacOS 10.13+ iOS 11+ | Metal  | Intel / Apple A13+ / M1+                   | ✅    | —       |
|                      |        | Apple A9+                                  | ❌      | ✅     |
| Linux                | Vulkan | Intel 6XXX+ / AMD GCN 2+  / NVIDIA 6XX+    | ✅    | —       |
|                      |        | Intel 4XXX+                                | ❌      | ✅     |
| Android              | Vulkan | All                                        | ❌      | ✅     |

Footnotes:
- ✅ Supported
- 🚧 In Progress
- ❌ Unsupported
- — Modern Profile Used
- Intel 6XXX = Skylake
- Intel 4XXX = Haswell
- Intel 2XXX = Sandy Bridge
- AMD GCN 2 = Rx 200+, RX 5000+
- Apple A9 = iPhone 6S, iPad 5th Gen
- Apple A13 = iPhone 11, iPad 9th Gen

# Purpose

`rend3` tries to fulfill the following usecases:
 1. Games and visualizations that need a customizable, and efficient
renderer.  2. Small projects that just want to put objects on screen, but
want lighting and effects.  3. A small cog in a big machine: a renderer
doesn't interfere with the rest of the program.

`rend3` is not:
 1. A framework or engine. It does not include all the parts needed to make
an advanced game or simulation nor care how you structure     your program.
I do have plans for a `rend3-util` (or similar) crate that is a very basic
framework for the second use case listed above.

# GPU Culling

On Vulkan and DX12 "gpu mode" is enabled by default, which uses modern
bindless resources and gpu-based culling. This reduces CPU load and allows
significantly more powerful culling.

# Future Plans

I have grand plans for this library. An overview can be found in the issue
tracker under the [enhancement] label.

[enhancement]: https://github.com/BVE-Reborn/rend3/labels/enhancement

# Matrix Chatroom

We have a matrix chatroom that you can come and join if you want to chat
about using rend3 or developing it:

[![Matrix](https://img.shields.io/static/v1?label=rend3%20dev&message=%23rend3&color=blueviolet&logo=matrix)](https://matrix.to/#/#rend3:matrix.org)
[![Matrix](https://img.shields.io/static/v1?label=rend3%20users&message=%23rend3-users&color=blueviolet&logo=matrix)](https://matrix.to/#/#rend3-users:matrix.org)

If discord is more your style, our meta project has a channel which mirrors
the matrix rooms:

[![Discord](https://img.shields.io/discord/451037457475960852?color=7289DA&label=discord)](https://discord.gg/mjxXTVzaDg)

# Helping Out

We welcome all contributions and ideas. If you want to participate or have
ideas for this library, we'd love to hear them!

License: MIT OR Apache-2.0 OR Zlib
