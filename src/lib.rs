
mod node_shape;

use node_shape::NodeShape;

use std::{fs, error::Error, collections::BTreeMap, time::Instant};

use eframe::{CreationContext, App};
use egui::{Pos2, Vec2, ahash::HashMap, Slider};
use egui_graphs::{add_node, add_edge, GraphView, SettingsInteraction, SettingsStyle, SettingsNavigation, add_node_custom, Node, Edge};
use petgraph::{stable_graph::StableGraph, adj::NodeIndex, Directed, visit::IntoNeighbors};
use rand::Rng;
use serde::Deserialize;


#[derive(Deserialize, Clone, Debug)]
struct NodeData {
    name: String,
    references: Vec<String>,
}

#[derive(Clone, Debug)]
struct NodePayload {
    name: String,
    vel: Vec2,
}

impl NodePayload {
    pub fn new(name: String) -> Self {
        Self {
            name,
            vel: Vec2::ZERO,
        }
    }
}

type G = egui_graphs::Graph<NodePayload, (), Directed, u32, NodeShape>;

struct ForceSettings {
    b_force: f32,
    r_force: f32,
    e_force: f32,
    stiffness: f32
}

impl Default for ForceSettings {
    fn default() -> Self {
        Self {
            b_force: 0.002,
            r_force: 400.,
            e_force: 0.001,
            stiffness: 0.2
        }
    }
}

pub struct MApp {
    g: G,
    last_update: Option<Instant>,
    force_settings: ForceSettings
}



impl MApp {
    pub fn new(_: &CreationContext<'_>) -> Self {
        let g = load_graph("add_comm.json");
        Self { g, last_update: None, force_settings: Default::default() }
    }
    fn simulate_force_graph(&mut self, dt: f32) {
        let indices = self.g.g.node_indices().collect::<Vec<_>>();

        let neighbors = indices.iter().map(|&ind| {
            let neigh = self.g.g.neighbors(ind).collect::<Vec<_>>();
            (ind, neigh)
        }).collect::<HashMap<_, _>>();

        for &ni in &indices {
            for &oni in &indices {
                if ni == oni {
                    continue;
                }
                let pos = self.g.node(ni).unwrap().location();
                let opos = self.g.node(oni).unwrap().location();

                let dir = opos-pos;
                let dis = dir.length();
                let dir = dir.normalized();

                let bacc = (self.force_settings.b_force * dis * dis);
                let bacc = bacc.min(1.);

                let racc = -(self.force_settings.r_force / dis);

                let eacc = self.force_settings.e_force * dis * dis;

                let tot_acc = bacc+racc + if neighbors[&ni].contains(&oni) { eacc } else {0.};

                let cvel = self.g.node_mut(ni).unwrap().payload().vel;
                self.g.node_mut(ni).unwrap().payload_mut().vel = cvel - (cvel*self.force_settings.stiffness*dt) + tot_acc * dt * dir;
                self.g.node_mut(ni).unwrap().set_location(pos + cvel * dt);
            }
        }
    } 
    fn draw_ui(&mut self, ctx: &eframe::egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let interaction_settings = &SettingsInteraction::new()
                .with_dragging_enabled(true)
                .with_node_clicking_enabled(true)
                .with_edge_clicking_enabled(true)
                .with_edge_selection_enabled(true)
                .with_node_selection_enabled(true);
                

            let style_settings = &SettingsStyle::new().with_labels_always(true);
            let navigations_settings = &SettingsNavigation::new().with_zoom_and_pan_enabled(true).with_fit_to_screen_enabled(false);
            ui.add(&mut GraphView::new(&mut self.g).with_styles(style_settings).with_navigations(navigations_settings).with_interactions(interaction_settings));
        });
        egui::SidePanel::new(egui::panel::Side::Right, "Settings").show(ctx, |ui| {
            ui.label("Edge force");
            ui.add(Slider::new(&mut self.force_settings.e_force, (0.000001)..=(0.001)));
            ui.label("Bounding force");
            ui.add(Slider::new(&mut self.force_settings.b_force, (0.000001)..=(0.1)));
            ui.label("Repulsive force");
            ui.add(Slider::new(&mut self.force_settings.r_force, (10.)..=(1000.)));
            ui.label("Stifness");
            ui.add(Slider::new(&mut self.force_settings.stiffness, (0.)..=1.));
        });
    }
}

impl App for MApp {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        let dt = self.last_update.unwrap_or(Instant::now()).elapsed().as_secs_f32();
        self.simulate_force_graph(dt);
        self.last_update = Some(Instant::now());
        self.draw_ui(ctx);
    }
}

fn load_graph(path: &str) -> G {
    let nodes = serde_json::from_str::<Vec<NodeData>>(&fs::read_to_string(path).unwrap()).unwrap();
    let mut sg = StableGraph::new();


    let nodes = nodes.into_iter().map(|node| {
        let ind =  sg.add_node(Node::new(NodePayload::new(node.name.clone())).with_label(node.name.clone()));
        sg.node_weight_mut(ind).unwrap().bind(ind, random_location(200.));

        (node.name.clone(), (ind, node))
    }).collect::<BTreeMap<String, (_, NodeData)>>();

    for (_, (ind, data)) in &nodes {
        for reference in &data.references {
            let ind = sg.add_edge(*ind, nodes[reference].0, Edge::new(()));
            sg.edge_weight_mut(ind).unwrap().bind(ind, 1);
        }
    }

    let g = G::new(sg);

    g
}

fn random_location(size: f32) -> Pos2 {
    let mut rng = rand::thread_rng();
    Pos2::new(rng.gen_range(0. ..size), rng.gen_range(0. ..size))
}