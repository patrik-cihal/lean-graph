use std::f32::consts::PI;

use egui::{
    epaint::{CircleShape, TextShape},
    FontFamily, FontId, Pos2, Shape, Stroke, Vec2,
};
use egui_graphs::{DisplayNode, DrawContext, NodeProps};
use petgraph::{stable_graph::IndexType, EdgeType};

use crate::{col_ft, ConstType, NodePayload};

#[derive(Clone, Debug)]
pub struct NodeShape {
    pub pos: Pos2,

    pub selected: bool,

    pub name: String,
    const_type: ConstType,

    /// Shape defined property
    pub radius: f32,
    color: [f32; 3],
}

impl From<NodeProps<NodePayload>> for NodeShape {
    fn from(node_props: NodeProps<NodePayload>) -> Self {
        NodeShape {
            pos: node_props.location,
            selected: node_props.selected,
            name: node_props.payload.name,

            radius: 10. * node_props.payload.size,
            color: node_props.payload.color,
            const_type: node_props.payload.const_type,
        }
    }
}

impl<E: Clone, Ty: EdgeType, Ix: IndexType> DisplayNode<NodePayload, E, Ty, Ix> for NodeShape {
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
        let color = if ctx.ctx.style().visuals.dark_mode {
            col_ft(self.color.map(|x| 1. - 2. * x))
        } else {
            col_ft(self.color)
        };
        let text_color = style.text_color();

        let center = ctx.meta.canvas_to_screen_pos(self.pos);
        let radius = ctx.meta.canvas_to_screen_size(self.radius);
        let get_n_polygon = |n: usize| {
            let step = 2. * PI / n as f32;
            (0..n)
                .map(|i| {
                    let ang = i as f32 * step;
                    let dir = Vec2::angled(ang);
                    Pos2::from(center + dir * radius)
                })
                .collect::<Vec<_>>()
        };
        let no_stroke = Stroke::new(0., color);
        let shape = match self.const_type {
            ConstType::Theorem => Shape::convex_polygon(get_n_polygon(5), color, no_stroke),
            ConstType::Definition => Shape::convex_polygon(get_n_polygon(3), color, no_stroke),
            ConstType::Axiom => CircleShape {
                center,
                radius,
                fill: color,
                stroke: Stroke::default(),
            }
            .into(),
            ConstType::Other => Shape::convex_polygon(get_n_polygon(4), color, no_stroke),
        };

        res.push(shape.into());

        let galley = ctx.ctx.fonts(|f| {
            f.layout_no_wrap(
                self.name.clone(),
                FontId::new(radius, FontFamily::Monospace),
                text_color,
            )
        });

        // display label centered over the circle
        let label_pos = Pos2::new(center.x - galley.size().x / 2., center.y - radius * 2.);

        let label_shape = TextShape::new(label_pos, galley);
        res.push(label_shape.into());

        res
    }

    fn update(&mut self, state: &NodeProps<NodePayload>) {
        self.pos = state.location;
        self.pos = state.location;
        self.selected = state.selected;
        self.name = state.payload.name.clone();
        self.color = state.payload.comp_color();
    }
}

fn closest_point_on_circle(center: Pos2, radius: f32, dir: Vec2) -> Pos2 {
    center + dir.normalized() * radius
}

fn is_inside_circle(center: Pos2, radius: f32, pos: Pos2) -> bool {
    let dir = pos - center;
    dir.length() <= radius
}
