//! Debug lines pass

use super::*;
use amethyst_core::cgmath::{Matrix4, One};
use amethyst_core::specs::{Join, Read, ReadStorage, Write, WriteStorage};
use amethyst_core::transform::GlobalTransform;
use cam::{ActiveCamera, Camera};
use debug_drawing::{DebugLine, DebugLines, DebugLinesComponent};
use error::Result;
use gfx::pso::buffer::ElemStride;
use gfx::Primitive;
use mesh::Mesh;
use pass::util::{get_camera, set_attribute_buffers, set_vertex_args, setup_vertex_args};
use pipe::pass::{Pass, PassData};
use pipe::{DepthMode, Effect, NewEffect};
use std::marker::PhantomData;
use types::{Encoder, Factory};
use vertex::{Color, Normal, Position, Query};

/// Draw several simple lines for debugging
///
/// See the [crate level documentation](index.html) for information about interleaved and separate
/// passes.
///
/// # Type Parameters:
///
/// * `V`: `VertexFormat`
#[derive(Derivative, Clone, Debug, PartialEq)]
#[derivative(Default(bound = "V: Query<(Position, Color, Normal)>"))]
pub struct DrawDebugLines<V> {
    _pd: PhantomData<V>,
}

impl<V> DrawDebugLines<V>
where
    V: Query<(Position, Color, Normal)>,
{
    /// Create instance of `DrawDebugLines` pass
    pub fn new() -> Self {
        Default::default()
    }
}

impl<'a, V> PassData<'a> for DrawDebugLines<V>
where
    V: Query<(Position, Color, Normal)>,
{
    type Data = (
        Option<Read<'a, ActiveCamera>>,
        ReadStorage<'a, Camera>,
        ReadStorage<'a, GlobalTransform>,
        WriteStorage<'a, DebugLinesComponent>, // DebugLines components
        Option<Write<'a, DebugLines>>,         // DebugLines resource
    );
}

impl<V> Pass for DrawDebugLines<V>
where
    V: Query<(Position, Color, Normal)>,
{
    fn compile(&mut self, effect: NewEffect) -> Result<Effect> {
        debug!("Building debug lines pass");
        let mut builder = effect.geom(VERT_SRC, GEOM_SRC, FRAG_SRC);

        debug!("Effect compiled, adding vertex/uniform buffers");
        builder.with_raw_vertex_buffer(V::QUERIED_ATTRIBUTES, V::size() as ElemStride, 0);

        setup_vertex_args(&mut builder);
        builder.with_raw_global("camera_position");
        builder.with_primitive_type(Primitive::PointList);
        builder.with_output("color", Some(DepthMode::LessEqualWrite));

        builder.build()
    }

    fn apply<'a, 'b: 'a>(
        &'a mut self,
        encoder: &mut Encoder,
        effect: &mut Effect,
        mut factory: Factory,
        (active, camera, global, lines_components, lines_resource): <Self as PassData<'a>>::Data,
    ) {
        trace!("Drawing debug lines pass");
        let debug_lines = {
            let mut lines = Vec::<DebugLine>::new();

            for debug_lines_component in (&lines_components).join() {
                lines.extend(&debug_lines_component.lines);
            }

            if let Some(mut lines_resource) = lines_resource {
                lines.append(&mut lines_resource.lines);
            };

            lines
        };

        let camera = get_camera(active, &camera, &global);
        effect.update_global(
            "camera_position",
            camera
                .as_ref()
                .map(|&(_, ref trans)| [trans.0[3][0], trans.0[3][1], trans.0[3][2]])
                .unwrap_or([0.0; 3]),
        );

        let mesh = Mesh::build(debug_lines)
            .build(&mut factory)
            .expect("Failed to create debug lines mesh");

        if !set_attribute_buffers(effect, &mesh, &[V::QUERIED_ATTRIBUTES]) {
            effect.clear();
            return;
        }

        set_vertex_args(effect, encoder, camera, &GlobalTransform(Matrix4::one()));

        effect.draw(mesh.slice(), encoder);
        effect.clear();
    }
}
