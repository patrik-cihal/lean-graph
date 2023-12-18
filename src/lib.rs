mod edge_shape;
mod node_shape;

use edge_shape::EdgeShape;
use node_shape::NodeShape;
use rfd::AsyncFileDialog;

const STATIC_JSON_FILES: [&str; 2] = ["Nat.zero_add.json", "Nat.prime_of_coprime"];
pub const SERVER_ADDR: &str = "http://localhost:8080";

use std::{
    collections::{BTreeMap, HashMap},
    future::Future,
    sync::{Arc, RwLock},
    time::Duration,
};

use eframe::{App, CreationContext};
use egui::{Color32, Pos2, Slider, Vec2};
use egui_graphs::{Edge, GraphView, Node, SettingsInteraction, SettingsNavigation, SettingsStyle};
use petgraph::{stable_graph::StableGraph, visit::IntoNeighbors, Directed};
use rand::{random, Rng};
use serde::Deserialize;

pub fn now() -> std::time::Duration {
    std::time::Duration::from_millis(chrono::Local::now().timestamp_millis() as u64)
}

fn col_ft(c: [f32; 3]) -> Color32 {
    Color32::from_rgb(
        (c[0] * 256.) as u8,
        (c[1] * 256.) as u8,
        (c[2] * 256.) as u8,
    )
}

#[derive(Deserialize, Clone, Debug, PartialEq, PartialOrd, Ord, Eq)]
enum ConstType {
    Theorem,
    Definition,
    Axiom,
    Other,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct NodeData {
    name: String,
    references: Vec<String>,
    const_type: ConstType,
}

#[derive(Clone, Debug)]
struct NodePayload {
    name: String,
    vel: Vec2,
    color: [f32; 3],
    comp_color: ([f32; 3], f32),
    const_type: ConstType,
    size: f32,
}

fn random_node_color() -> [f32; 3] {
    [0.; 3].map(|_| random::<f32>() / 3.)
}

impl From<&NodeData> for NodePayload {
    fn from(value: &NodeData) -> Self {
        Self {
            name: value.name.clone(),
            const_type: value.const_type.clone(),
            color: random_node_color(),
            comp_color: Default::default(),
            vel: Vec2::ZERO,
            size: ((value.references.len() + 1) as f32).sqrt(),
        }
    }
}

impl NodePayload {
    pub fn comp_color(&self) -> [f32; 3] {
        self.comp_color.0.map(|x| x / self.comp_color.1)
    }
    pub fn mass(&self) -> f32 {
        self.size
    }
}

type G = egui_graphs::Graph<NodePayload, (), Directed, u32, NodeShape, EdgeShape>;

struct ForceSettings {
    b_force: f32,
    r_force: f32,
    e_force: f32,
    stiffness: f32,
}

impl Default for ForceSettings {
    fn default() -> Self {
        Self {
            b_force: 0.0005,
            r_force: 400.,
            e_force: 0.001,
            stiffness: 0.5,
        }
    }
}

struct ColoringSettings {
    color_loss: f32,
}

impl Default for ColoringSettings {
    fn default() -> Self {
        Self { color_loss: 0.8 }
    }
}

pub struct MApp {
    g: Arc<RwLock<G>>,
    g_updated: Arc<RwLock<bool>>,
    fg: G,
    last_update: Option<Duration>,
    force_settings: ForceSettings,
    node_type_filter: BTreeMap<ConstType, bool>,
    outer_edge_cnt_filter: usize,
    coloring_settings: ColoringSettings,
}

impl MApp {
    pub fn new(_: &CreationContext<'_>, default_file_raw: String) -> Self {
        let g = load_graph(default_file_raw);
        let mut node_type_filter = BTreeMap::new();

        node_type_filter.insert(ConstType::Axiom, true);
        node_type_filter.insert(ConstType::Definition, true);
        node_type_filter.insert(ConstType::Theorem, true);
        node_type_filter.insert(ConstType::Other, false);

        Self {
            g: Arc::new(RwLock::new(g.clone())),
            g_updated: Default::default(),
            last_update: None,
            force_settings: Default::default(),
            node_type_filter,
            fg: g,
            outer_edge_cnt_filter: 10,
            coloring_settings: Default::default(),
        }
    }
    fn color_nodes(&mut self) {
        let node_indices = self.fg.g.node_indices().collect::<Vec<_>>();
        for &ni in &node_indices {
            self.fg.g[ni].payload_mut().comp_color = Default::default();
        }

        // get node_indices as topological sort

        let mut out_degree = HashMap::new();
        let mut rev_neighbors = HashMap::new();
        for &ni in &node_indices {
            *out_degree.entry(ni).or_insert(0) += self.fg.g.neighbors(ni).count();
            for oni in self.fg.g.neighbors(ni).collect::<Vec<_>>() {
                rev_neighbors.entry(oni).or_insert(vec![]).push(ni);
            }
        }

        let mut stack = vec![];
        for &ni in &node_indices {
            if *out_degree.entry(ni).or_insert(0) == 0 {
                stack.push(ni);
            }
        }

        let mut topo_sort = vec![];

        while let Some(cur) = stack.pop() {
            topo_sort.push(cur);
            for oni in rev_neighbors.entry(cur).or_insert(vec![]).clone() {
                *out_degree.get_mut(&oni).unwrap() -= 1;
                if out_degree[&oni] == 0 {
                    stack.push(oni);
                }
            }
        }

        for &ni in &topo_sort {
            let color = self.fg.g.node_weight(ni).unwrap().payload().color;
            let size = self.fg.g[ni].payload().size;
            // add cur color to comp color
            let comp_color = self.fg.g[ni].payload_mut().comp_color;
            self.fg.g[ni].payload_mut().comp_color.0 = [
                comp_color.0[0] + color[0] * size,
                comp_color.0[1] + color[1] * size,
                comp_color.0[2] + color[2] * size,
            ];
            self.fg.g[ni].payload_mut().comp_color.1 += size;
            let comp_color = self.fg.g[ni].payload_mut().comp_color;

            // for each neighbor add my own comp color with some loss based on a constant
            for &oni in &rev_neighbors[&ni] {
                for i in 0..3 {
                    self.fg.g[oni].payload_mut().comp_color.0[i] +=
                        comp_color.0[i] * self.coloring_settings.color_loss;
                }
                self.fg.g[oni].payload_mut().comp_color.1 +=
                    comp_color.1 * self.coloring_settings.color_loss;
            }
        }
    }
    fn simulate_force_graph(&mut self, dt: f32) {
        let indices = self.fg.g.node_indices().collect::<Vec<_>>();

        let neighbors = indices
            .iter()
            .map(|&ind| {
                let neigh = self.fg.g.neighbors(ind).collect::<Vec<_>>();
                (ind, neigh)
            })
            .collect::<HashMap<_, _>>();

        for &ni in &indices {
            for &oni in &indices {
                if ni == oni {
                    continue;
                }
                let pos = self.fg.node(ni).unwrap().location();
                let opos = self.fg.node(oni).unwrap().location();

                let dir = opos - pos;
                let dis = dir.length();
                let dir = dir.normalized();

                let bacc = self.force_settings.b_force * dis;

                let racc = -(self.force_settings.r_force / (dis.sqrt()));

                let eacc = self.force_settings.e_force * dis * dis;

                let mr = self.fg.g[oni].payload().mass() / self.fg.g[ni].payload().mass();

                let tot_acc = mr
                    * (bacc
                        + racc
                        + if neighbors[&ni].contains(&oni) {
                            eacc
                        } else {
                            0.
                        });

                let cvel = self.fg.node_mut(ni).unwrap().payload().vel;
                self.fg.node_mut(ni).unwrap().payload_mut().vel =
                    cvel - (cvel * self.force_settings.stiffness * dt) + tot_acc * 0.01 * dt * dir;
                self.fg.node_mut(ni).unwrap().set_location(pos + cvel * dt);
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
                .with_node_selection_enabled(true)
                .with_node_selection_multi_enabled(true);

            let style_settings = &SettingsStyle::new().with_labels_always(true);
            let navigations_settings = &SettingsNavigation::new()
                .with_zoom_and_pan_enabled(true)
                .with_fit_to_screen_enabled(false);

            ui.add(
                &mut GraphView::new(&mut self.fg)
                    .with_styles(style_settings)
                    .with_navigations(navigations_settings)
                    .with_interactions(interaction_settings),
            );
        });
        egui::SidePanel::new(egui::panel::Side::Right, "Settings").show(ctx, |ui| {
            ui.collapsing("File settings", |ui| {
                #[cfg(target_arch = "wasm32")]
                ui.collapsing("Open from server", |ui| {
                    for &server_file_name in &STATIC_JSON_FILES {
                        if ui.button(server_file_name).clicked() {
                            // download file from server and set it as current graph
                            let gc = self.g.clone();
                            let guc = self.g_updated.clone();

                            spawn_local(async move {
                                let ng_raw = read_graph_url(&format!(
                                    "{SERVER_ADDR}/static/{server_file_name}"
                                ))
                                .await
                                .unwrap();
                                let ng = load_graph(ng_raw);

                                *gc.write().unwrap() = ng.clone();
                                *guc.write().unwrap() = true;
                            })
                        }
                    }
                });
                if ui.button("Open local").clicked() {
                    let gc = self.g.clone();
                    let guc = self.g_updated.clone();
                    spawn_local(async move {
                        let Some(ng_raw) = read_graph_file_dialog().await else {
                            return;
                        };
                        let ng = load_graph(ng_raw);
                        *gc.write().unwrap() = ng.clone();
                        *guc.write().unwrap() = true;
                    });
                }
                #[cfg(target_arch = "wasm32")]
                if ui.button("Download dependency extractor").clicked() {
                    spawn_local(async move {
                        let Some(file_handle) = AsyncFileDialog::new()
                            .set_file_name("dep_extractor.lean")
                            .save_file()
                            .await
                        else {
                            return;
                        };
                        let data_raw = read_dep_extractor().await.unwrap();
                        file_handle.write(data_raw.as_bytes()).await.unwrap();
                    });
                }
            });

            ui.collapsing("Force simulation settings", |ui| {
                ui.label("Edge attraction");
                ui.add(Slider::new(
                    &mut self.force_settings.e_force,
                    (0.0)..=(0.002),
                ));
                ui.label("General bounding");
                ui.add(Slider::new(
                    &mut self.force_settings.b_force,
                    (0.0)..=(0.002),
                ));
                ui.label("Repulsion");
                ui.add(Slider::new(
                    &mut self.force_settings.r_force,
                    (10.)..=(1000.),
                ));
                ui.label("Stifness");
                ui.add(Slider::new(&mut self.force_settings.stiffness, (0.)..=1.));
            });
            ui.collapsing("Coloring settings", |ui| {
                ui.label("Node coloring loss");
                ui.add(Slider::new(
                    &mut self.coloring_settings.color_loss,
                    (0.0)..=1.0,
                ));
                if ui.button("Randomize colors").clicked() {
                    for ni in self.fg.g.node_indices().collect::<Vec<_>>() {
                        self.fg.g[ni].payload_mut().color = random_node_color();
                    }
                }
            });

            ui.collapsing("Filter", |ui| {
                ui.checkbox(
                    self.node_type_filter.get_mut(&ConstType::Axiom).unwrap(),
                    "Axioms",
                );
                ui.checkbox(
                    self.node_type_filter.get_mut(&ConstType::Theorem).unwrap(),
                    "Theorems",
                );
                ui.checkbox(
                    self.node_type_filter
                        .get_mut(&ConstType::Definition)
                        .unwrap(),
                    "Definitions",
                );
                ui.checkbox(
                    self.node_type_filter.get_mut(&ConstType::Other).unwrap(),
                    "Other",
                );
                ui.label("Max node out-degree");
                ui.add(Slider::new(&mut self.outer_edge_cnt_filter, 1..=1000));
            });
        });
    }

    fn update_filter_graph(&mut self) {
        let mut g = self.g.write().unwrap();
        if !*self.g_updated.read().unwrap() {
            for &ni in &self.fg.g.node_indices().collect::<Vec<_>>() {
                let cur_node = self.fg.g[ni].clone();
                *g.g.node_weight_mut(ni).unwrap() = cur_node;
            }
        }
        *self.g_updated.write().unwrap() = false;
        self.fg = G::new(g.g.filter_map(
            |ni, node| {
                if self.node_type_filter[&node.payload().const_type]
                    && g.g.neighbors(ni).count() <= self.outer_edge_cnt_filter
                {
                    Some(node.clone())
                } else {
                    None
                }
            },
            |_, edge| Some(edge.clone()),
        ));
    }
}

impl App for MApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.update_filter_graph();
        let ct = now();
        let dt = (ct.clone() - self.last_update.unwrap_or(ct)).as_secs_f32();
        self.simulate_force_graph(dt.min(0.05));
        self.last_update = Some(ct);
        self.color_nodes();
        self.draw_ui(ctx);
    }
}

fn load_graph(default_file_raw: String) -> G {
    let nodes = serde_json::from_str::<Vec<NodeData>>(&default_file_raw).unwrap();
    let mut sg = StableGraph::new();

    let spawn_radius = (nodes.len() as f32).sqrt() * 1000.;

    let nodes = nodes
        .into_iter()
        .map(|node| {
            let ind =
                sg.add_node(Node::new(NodePayload::from(&node)).with_label(node.name.clone()));
            sg.node_weight_mut(ind)
                .unwrap()
                .bind(ind, random_location(spawn_radius));

            (node.name.clone(), (ind, node))
        })
        .collect::<BTreeMap<String, (_, NodeData)>>();

    for (_, (ind, data)) in &nodes {
        for reference in &data.references {
            if let Some(node) = nodes.get(reference) {
                let ind = sg.add_edge(node.0, *ind, Edge::new(()));
                sg.edge_weight_mut(ind).unwrap().bind(ind, 1);
            }
        }
    }

    let g = G::new(sg);

    g
}

fn random_location(size: f32) -> Pos2 {
    let mut rng = rand::thread_rng();
    Pos2::new(rng.gen_range(0. ..size), rng.gen_range(0. ..size))
}

pub async fn read_graph_file_dialog() -> Option<String> {
    let Some(file_handle) = AsyncFileDialog::new()
        .add_filter("Json", &["json"])
        .pick_file()
        .await
    else {
        return None;
    };
    let data_raw = file_handle.read().await;
    Some(String::from_utf8(data_raw).unwrap())
}

pub async fn read_graph_url(url: &str) -> Result<String, reqwest::Error> {
    let resp = reqwest::get(url).await?;
    resp.error_for_status_ref()?;
    resp.text().await
}

pub async fn read_dep_extractor() -> Result<String, reqwest::Error> {
    let resp = reqwest::get(format!("{SERVER_ADDR}/static/dep_extractor.lean")).await?;
    resp.error_for_status_ref()?;
    resp.text().await
}

fn spawn_local<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(future);
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future);
    }
}
