//! Starter RenderGraph that can be easily extended.
//!
//! This is a fully put together pipeline to render with rend3. If you don't
//! need any customization, this should be drop in without worrying about it.
//!
//! In order to start customizing it, copy the contents of
//! [`BaseRenderGraph::add_to_graph`] into your own code and start modifying it.
//! This will allow you to insert your own routines and customize the behavior
//! of the existing routines.
//!
//! [`BaseRenderGraphIntermediateState`] intentionally has all of its members
//! public. If you want to change what rendergraph image things are rendering
//! to, or muck with any of the data in there, you are free to, and the
//! following routines will behave as you configure.

use std::{iter::zip, sync::Arc};

use glam::{UVec2, Vec4};
use rend3::{
    format_sso,
    graph::{
        DataHandle, InstructionEvaluationOutput, RenderGraph, RenderTargetDescriptor, RenderTargetHandle, ViewportRect,
    },
    managers::ShadowDesc,
    types::{SampleCount, TextureFormat, TextureUsages},
    Renderer, ShaderPreProcessor, INTERNAL_SHADOW_DEPTH_FORMAT,
};
use wgpu::{BindGroup, Buffer};

use crate::{
    common::{self, CameraIndex},
    culling,
    forward::RoutineAddToGraphArgs,
    pbr, skinning, skybox, tonemapping,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DepthTargets {
    pub single_sample_mipped: RenderTargetHandle,
    pub multi_sample: Option<RenderTargetHandle>,
}

impl DepthTargets {
    pub fn new(graph: &mut RenderGraph<'_>, resolution: UVec2, samples: SampleCount) -> Self {
        let single_sample_mipped = graph.add_render_target(RenderTargetDescriptor {
            label: Some("hdr depth".into()),
            resolution,
            depth: 1,
            mip_levels: None,
            samples: SampleCount::One,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        });

        let multi_sample = samples.needs_resolve().then(|| {
            graph.add_render_target(RenderTargetDescriptor {
                label: Some("hdr depth multisampled".into()),
                resolution,
                depth: 1,
                mip_levels: Some(1),
                samples,
                format: TextureFormat::Depth32Float,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            })
        });

        Self {
            single_sample_mipped,
            multi_sample,
        }
    }

    pub fn rendering_target(&self) -> RenderTargetHandle {
        self.multi_sample.unwrap_or(self.single_sample_mipped.set_mips(0..1))
    }
}

/// Starter RenderGraph.
///
/// See module for documentation.
pub struct BaseRenderGraph {
    pub interfaces: common::WholeFrameInterfaces,
    pub samplers: common::Samplers,
    pub gpu_culler: culling::GpuCuller,
    pub gpu_skinner: skinning::GpuSkinner,
}

impl BaseRenderGraph {
    pub fn new(renderer: &Arc<Renderer>, spp: &ShaderPreProcessor) -> Self {
        profiling::scope!("DefaultRenderGraphData::new");

        let interfaces = common::WholeFrameInterfaces::new(&renderer.device);

        let samplers = common::Samplers::new(&renderer.device);

        // TODO: Support more materials
        let gpu_culler = culling::GpuCuller::new::<pbr::PbrMaterial>(renderer, spp);

        let gpu_skinner = skinning::GpuSkinner::new(&renderer.device, spp);

        Self {
            interfaces,
            samplers,
            gpu_culler,
            gpu_skinner,
        }
    }

    /// Add this to the rendergraph. This is the function you should start
    /// customizing.
    #[allow(clippy::too_many_arguments)]
    pub fn add_to_graph<'node>(
        &'node self,
        graph: &mut RenderGraph<'node>,
        eval_output: &InstructionEvaluationOutput,
        pbr: &'node crate::pbr::PbrRoutine,
        skybox: Option<&'node crate::skybox::SkyboxRoutine>,
        tonemapping: &'node crate::tonemapping::TonemappingRoutine,
        target_texture: RenderTargetHandle,
        resolution: UVec2,
        samples: SampleCount,
        ambient: Vec4,
        clear_color: Vec4,
    ) {
        // Create the data and handles for the graph.
        let state = BaseRenderGraphIntermediateState::new(graph, eval_output, resolution, samples);

        // Clear the shadow map.
        state.clear_shadow(graph);

        // Prepare all the uniforms that all shaders need access to.
        state.create_frame_uniforms(graph, self, ambient, resolution);

        // Perform compute based skinning.
        state.skinning(graph, self);

        // Upload the uniforms for the objects in the shadow pass.
        state.shadow_object_uniform_upload(graph, self, eval_output);
        // Perform culling for the objects in the shadow pass.
        state.pbr_shadow_culling(graph, self);

        // Render all the shadows to the shadow map.
        state.pbr_shadow_rendering(graph, pbr, &eval_output.shadows);

        // Clear the primary render target and depth target.
        state.clear(graph, clear_color);

        // Upload the uniforms for the objects in the forward pass.
        state.object_uniform_upload(graph, self, resolution, samples);

        // Do the first pass, rendering the predicted triangles from last frame.
        state.pbr_render_opaque_predicted_triangles(graph, pbr, samples);

        // Create the hi-z buffer.
        state.hi_z(graph, pbr, resolution);

        // Perform culling for the objects in the forward pass.
        //
        // The result of culling will be used to predict the visible triangles for
        // the next frame. It will also render all the triangles that were visible
        // but were not predicted last frame.
        state.pbr_culling(graph, self);

        // Do the second pass, rendering the residual triangles.
        state.pbr_render_opaque_residual_triangles(graph, pbr, samples);

        // Render the skybox.
        state.skybox(graph, skybox, samples);

        // Render all transparent objects.
        //
        // This _must_ happen after culling, as all transparent objects are
        // considered "residual".
        state.pbr_forward_rendering_transparent(graph, pbr, samples);

        // Tonemap the HDR inner buffer to the output buffer.
        state.tonemapping(graph, tonemapping, target_texture);
    }
}

/// Struct that globs all the information the [`BaseRenderGraph`] needs.
///
/// This is intentionally public so all this can be changed by the user if they
/// so desire.
pub struct BaseRenderGraphIntermediateState {
    pub pre_cull: DataHandle<Buffer>,
    pub shadow_cull: Vec<DataHandle<Arc<culling::DrawCallSet>>>,
    pub cull: DataHandle<Arc<culling::DrawCallSet>>,

    pub shadow_uniform_bg: DataHandle<BindGroup>,
    pub forward_uniform_bg: DataHandle<BindGroup>,
    pub shadow: RenderTargetHandle,
    pub color: RenderTargetHandle,
    pub resolve: Option<RenderTargetHandle>,
    pub depth: DepthTargets,
    pub pre_skinning_buffers: DataHandle<skinning::PreSkinningBuffers>,
}
impl BaseRenderGraphIntermediateState {
    /// Create the default setting for all state.
    pub fn new(
        graph: &mut RenderGraph<'_>,
        eval_output: &InstructionEvaluationOutput,
        resolution: UVec2,
        samples: SampleCount,
    ) -> Self {
        // We need to know how many shadows we need to render
        let shadow_count = eval_output.shadows.len();

        // Create global bind group information
        let shadow_uniform_bg = graph.add_data::<BindGroup>();
        let forward_uniform_bg = graph.add_data::<BindGroup>();

        // Shadow render target
        let shadow = graph.add_render_target(RenderTargetDescriptor {
            label: Some("shadow target".into()),
            resolution: eval_output.shadow_target_size,
            depth: 1,
            mip_levels: Some(1),
            samples: SampleCount::One,
            format: INTERNAL_SHADOW_DEPTH_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        });

        // Make the actual render targets we want to render to.
        let color = graph.add_render_target(RenderTargetDescriptor {
            label: Some("hdr color".into()),
            resolution,
            depth: 1,
            samples,
            mip_levels: Some(1),
            format: TextureFormat::Rgba16Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        });
        let resolve = samples.needs_resolve().then(|| {
            graph.add_render_target(RenderTargetDescriptor {
                label: Some("hdr resolve".into()),
                resolution,
                depth: 1,
                mip_levels: Some(1),
                samples: SampleCount::One,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            })
        });
        let depth = DepthTargets::new(graph, resolution, samples);

        let pre_skinning_buffers = graph.add_data::<skinning::PreSkinningBuffers>();

        Self {
            pre_cull: graph.add_data(),
            shadow_cull: {
                let mut shadows = Vec::with_capacity(shadow_count);
                shadows.resize_with(shadow_count, || graph.add_data());
                shadows
            },
            cull: graph.add_data(),

            shadow_uniform_bg,
            forward_uniform_bg,
            shadow,
            color,
            resolve,
            depth,
            pre_skinning_buffers,
        }
    }

    /// Create all the uniforms all the shaders in this graph need.
    pub fn create_frame_uniforms<'node>(
        &self,
        graph: &mut RenderGraph<'node>,
        base: &'node BaseRenderGraph,
        ambient: Vec4,
        resolution: UVec2,
    ) {
        crate::uniforms::add_to_graph(
            graph,
            self.shadow_uniform_bg,
            self.forward_uniform_bg,
            self.shadow,
            &base.interfaces,
            &base.samplers,
            ambient,
            resolution,
        );
    }
    pub fn shadow_object_uniform_upload<'node>(
        &self,
        graph: &mut RenderGraph<'node>,
        base: &'node BaseRenderGraph,
        eval_output: &InstructionEvaluationOutput,
    ) {
        for (shadow_index, shadow) in eval_output.shadows.iter().enumerate() {
            base.gpu_culler.add_object_uniform_upload_to_graph::<pbr::PbrMaterial>(
                graph,
                CameraIndex::Shadow(shadow_index as u32),
                UVec2::splat(shadow.map.size),
                SampleCount::One,
                &format_sso!("Shadow Culling S{}", shadow_index),
            );
        }
    }

    /// Does all shadow culling for the PBR materials.
    pub fn pbr_shadow_culling<'node>(&self, graph: &mut RenderGraph<'node>, base: &'node BaseRenderGraph) {
        for (shadow_index, &shadow_culled) in self.shadow_cull.iter().enumerate() {
            base.gpu_culler.add_culling_to_graph::<pbr::PbrMaterial>(
                graph,
                shadow_culled,
                self.shadow,
                CameraIndex::Shadow(shadow_index as u32),
                &format_sso!("Shadow Culling S{}", shadow_index),
            );
        }
    }

    pub fn skinning<'node>(&self, graph: &mut RenderGraph<'node>, base: &'node BaseRenderGraph) {
        skinning::add_skinning_to_graph(graph, &base.gpu_skinner);
    }

    pub fn object_uniform_upload<'node>(
        &self,
        graph: &mut RenderGraph<'node>,
        base: &'node BaseRenderGraph,
        resolution: UVec2,
        samples: SampleCount,
    ) {
        base.gpu_culler.add_object_uniform_upload_to_graph::<pbr::PbrMaterial>(
            graph,
            CameraIndex::Viewport,
            resolution,
            samples,
            "Uniform Bake",
        );
    }

    /// Does all culling for the forward PBR materials.
    pub fn pbr_culling<'node>(&self, graph: &mut RenderGraph<'node>, base: &'node BaseRenderGraph) {
        base.gpu_culler.add_culling_to_graph::<pbr::PbrMaterial>(
            graph,
            self.cull,
            self.depth.single_sample_mipped,
            CameraIndex::Viewport,
            "Primary Culling",
        );
    }

    /// Clear all the targets to their needed values
    pub fn clear_shadow(&self, graph: &mut RenderGraph<'_>) {
        crate::clear::add_clear_to_graph(graph, None, None, self.shadow, Vec4::ZERO, 0.0);
    }

    /// Clear all the targets to their needed values
    pub fn clear(&self, graph: &mut RenderGraph<'_>, clear_color: Vec4) {
        crate::clear::add_clear_to_graph(
            graph,
            Some(self.color),
            self.resolve,
            self.depth.rendering_target(),
            clear_color,
            0.0,
        );
    }

    /// Render all shadows for the PBR materials.
    pub fn pbr_shadow_rendering<'node>(
        &self,
        graph: &mut RenderGraph<'node>,
        pbr: &'node pbr::PbrRoutine,
        shadows: &[ShadowDesc],
    ) {
        let iter = zip(&self.shadow_cull, shadows);
        for (shadow_index, (shadow_cull, desc)) in iter.enumerate() {
            let routines = [&pbr.opaque_depth, &pbr.cutout_depth];
            for routine in routines {
                routine.add_forward_to_graph(RoutineAddToGraphArgs {
                    graph,
                    whole_frame_uniform_bg: self.shadow_uniform_bg,
                    culling_output_handle: Some(*shadow_cull),
                    per_material: &pbr.per_material,
                    extra_bgs: None,
                    label: &format!("pbr shadow renderering S{shadow_index}"),
                    samples: SampleCount::One,
                    camera: CameraIndex::Shadow(shadow_index as u32),
                    color: None,
                    resolve: None,
                    depth: self
                        .shadow
                        .set_viewport(ViewportRect::new(desc.map.offset, UVec2::splat(desc.map.size))),
                });
            }
        }
    }

    /// Render the skybox.
    pub fn skybox<'node>(
        &self,
        graph: &mut RenderGraph<'node>,
        skybox: Option<&'node skybox::SkyboxRoutine>,
        samples: SampleCount,
    ) {
        if let Some(skybox) = skybox {
            skybox.add_to_graph(
                graph,
                self.color,
                self.resolve,
                self.depth.rendering_target(),
                self.forward_uniform_bg,
                samples,
            );
        }
    }

    /// Render the PBR materials.
    pub fn pbr_render_opaque_predicted_triangles<'node>(
        &self,
        graph: &mut RenderGraph<'node>,
        pbr: &'node pbr::PbrRoutine,
        samples: SampleCount,
    ) {
        let routines = [&pbr.opaque_routine, &pbr.cutout_routine];
        for routine in routines {
            routine.add_forward_to_graph(RoutineAddToGraphArgs {
                graph,
                whole_frame_uniform_bg: self.forward_uniform_bg,
                culling_output_handle: None,
                per_material: &pbr.per_material,
                extra_bgs: None,
                label: "PBR Forward Pass 1",
                samples,
                camera: CameraIndex::Viewport,
                color: Some(self.color),
                resolve: self.resolve,
                depth: self.depth.rendering_target(),
            });
        }
    }

    /// Render the PBR materials.
    pub fn pbr_render_opaque_residual_triangles<'node>(
        &self,
        graph: &mut RenderGraph<'node>,
        pbr: &'node pbr::PbrRoutine,
        samples: SampleCount,
    ) {
        let routines = [&pbr.opaque_routine, &pbr.cutout_routine];
        for routine in routines {
            routine.add_forward_to_graph(RoutineAddToGraphArgs {
                graph,
                whole_frame_uniform_bg: self.forward_uniform_bg,
                culling_output_handle: Some(self.cull),
                per_material: &pbr.per_material,
                extra_bgs: None,
                label: "PBR Forward Pass 2",
                samples,
                camera: CameraIndex::Viewport,
                color: Some(self.color),
                resolve: self.resolve,
                depth: self.depth.rendering_target(),
            });
        }
    }

    /// Render the PBR materials.
    pub fn pbr_forward_rendering_transparent<'node>(
        &self,
        graph: &mut RenderGraph<'node>,
        pbr: &'node pbr::PbrRoutine,
        samples: SampleCount,
    ) {
        pbr.blend_routine.add_forward_to_graph(RoutineAddToGraphArgs {
            graph,
            whole_frame_uniform_bg: self.forward_uniform_bg,
            culling_output_handle: Some(self.cull),
            per_material: &pbr.per_material,
            extra_bgs: None,
            label: "PBR Forward",
            camera: CameraIndex::Viewport,
            samples,
            color: Some(self.color),
            resolve: self.resolve,
            depth: self.depth.rendering_target(),
        });
    }

    pub fn hi_z<'node>(&self, graph: &mut RenderGraph<'node>, pbr: &'node pbr::PbrRoutine, resolution: UVec2) {
        pbr.hi_z.add_hi_z_to_graph(graph, self.depth, resolution);
    }

    /// Tonemap onto the given render target.
    pub fn tonemapping<'node>(
        &self,
        graph: &mut RenderGraph<'node>,
        tonemapping: &'node tonemapping::TonemappingRoutine,
        target: RenderTargetHandle,
    ) {
        tonemapping.add_to_graph(
            graph,
            self.resolve.unwrap_or(self.color),
            target,
            self.forward_uniform_bg,
        );
    }
}
