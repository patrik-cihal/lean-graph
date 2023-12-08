use egui::{Pos2, Vec2, Shape, epaint::{CircleShape, TextShape}, Stroke, FontId, FontFamily};
use egui_graphs::{NodeProps, DisplayNode, DrawContext, DefaultNodeShape};
use petgraph::{EdgeType, stable_graph::IndexType};

use crate::NodePayload;

#[derive(Clone, Debug)]
pub struct NodeShape {
    pub pos: Pos2,

    pub selected: bool,

    pub name: String,

    /// Shape defined property
    pub radius: f32,
}

impl From<NodeProps<NodePayload>> for NodeShape {
    fn from(node_props: NodeProps<NodePayload>) -> Self {
        NodeShape {
            pos: node_props.location,
            selected: node_props.selected,
            name: node_props.payload.name,

            radius: 5.0,
        }
    }
}

impl<E: Clone, Ty: EdgeType, Ix: IndexType> DisplayNode<NodePayload, E, Ty, Ix>
    for NodeShape
{
    fn is_inside(&self, pos: Pos2) -> bool {
        is_inside_circle(self.pos, self.radius, pos)
    }

    fn closest_boundary_point(&self, dir: Vec2) -> Pos2 {
        closest_point_on_circle(self.pos, self.radius, dir)
    }

    fn shapes(&mut self, ctx: &DrawContext) -> Vec<Shape> {
        let mut res = Vec::with_capacity(2);

        let is_interacted = self.selected;

        let style = match is_interacted {
            true => ctx.ctx.style().visuals.widgets.active,
            false => ctx.ctx.style().visuals.widgets.inactive,
        };
        let color = style.fg_stroke.color;

        let circle_center = ctx.meta.canvas_to_screen_pos(self.pos);
        let circle_radius = ctx.meta.canvas_to_screen_size(self.radius);
        let circle_shape = CircleShape {
            center: circle_center,
            radius: circle_radius,
            fill: color,
            stroke: Stroke::default(),
        };
        res.push(circle_shape.into());

        let galley = ctx.ctx.fonts(|f| {
            f.layout_no_wrap(
                self.name.clone(),
                FontId::new(circle_radius, FontFamily::Monospace),
                color,
            )
        });

        // display label centered over the circle
        let label_pos = Pos2::new(
            circle_center.x - galley.size().x / 2.,
            circle_center.y - circle_radius * 2.,
        );

        let label_shape = TextShape::new(label_pos, galley);
        res.push(label_shape.into());

        res
    }

    fn update(&mut self, state: &NodeProps<NodePayload>) {
        self.pos = state.location;
        self.pos = state.location;
        self.selected = state.selected;
        self.name = state.payload.name.clone();
    }
}

fn closest_point_on_circle(center: Pos2, radius: f32, dir: Vec2) -> Pos2 {
    center + dir.normalized() * radius
}

fn is_inside_circle(center: Pos2, radius: f32, pos: Pos2) -> bool {
    let dir = pos - center;
    dir.length() <= radius
}