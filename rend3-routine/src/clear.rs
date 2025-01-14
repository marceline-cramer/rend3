use glam::Vec4;
use rend3::graph::{
    NodeResourceUsage, RenderGraph, RenderPassDepthTarget, RenderPassTarget, RenderPassTargets, RenderTargetHandle,
};

/// Uses the given targets to create a node which merely sets the clear color to what we want.
///
/// While not strictly needed as the first pass using a target will get its clear color followed,
/// it makes it a lot easier to udnerstand where the clear is coming from.
pub fn add_clear_to_graph(
    graph: &mut RenderGraph<'_>,
    color: Option<RenderTargetHandle>,
    resolve: Option<RenderTargetHandle>,
    depth: RenderTargetHandle,
    clear_color: Vec4,
    depth_clear: f32,
) {
    let mut builder = graph.add_node("Clear");

    let hdr_color_handle = builder.add_optional_render_target(color, NodeResourceUsage::Output);
    let hdr_resolve = builder.add_optional_render_target(resolve, NodeResourceUsage::Output);
    let hdr_depth_handle = builder.add_render_target(depth, NodeResourceUsage::Output);

    let _rpass_handle = builder.add_renderpass(RenderPassTargets {
        targets: if let Some(hdr_color_handle) = hdr_color_handle {
            vec![RenderPassTarget {
                color: hdr_color_handle,
                clear: wgpu::Color {
                    r: clear_color.x as f64,
                    g: clear_color.y as f64,
                    b: clear_color.z as f64,
                    a: clear_color.w as f64,
                },
                resolve: hdr_resolve,
            }]
        } else {
            vec![]
        },
        depth_stencil: Some(RenderPassDepthTarget {
            target: hdr_depth_handle,
            depth_clear: Some(depth_clear),
            stencil_clear: None,
        }),
    });

    builder.build(|_| ())
}
