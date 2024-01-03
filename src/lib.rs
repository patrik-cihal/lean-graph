mod edge_shape;
mod node_shape;

use edge_shape::EdgeShape;
use node_shape::NodeShape;
use rfd::AsyncFileDialog;

const STATIC_JSON_FILES: [&str; 7] = ["Nat.zero_add.json", "Nat.prime_of_coprime.json", "Topology.json", "Cardinal.cantor.json", "Continuous.deriv_integral.json", "fermatLastTheoremFour.json", "PFR_conjecture.json"];
pub const SERVER_ADDR: &str = "https://lean-graph.com";

use std::{
    collections::{BTreeMap, HashMap, BinaryHeap},
    future::Future,
    sync::{Arc, RwLock},
    time::Duration, f32::consts::PI, cmp::Reverse,
};

use eframe::{App, CreationContext};
use egui::{Color32, Pos2, Slider, Vec2, Visuals, Hyperlink, emath::align::center_size_in_rect};
use egui_graphs::{Edge, GraphView, Node, SettingsInteraction, SettingsNavigation, SettingsStyle};
use petgraph::{stable_graph::StableGraph, graph::NodeIndex, EdgeType};
use rand::{random, Rng};
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Clone)]
pub struct Directed {}

impl EdgeType for Directed {
    fn is_directed() -> bool {
        true 
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, PartialOrd, Ord, Eq)]
enum ConstCategory {
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
    const_category: ConstCategory,
    const_type: String
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct NodePayload {
    name: String,
    vel: Vec2,
    color: [f32; 3],
    comp_color: ([f32; 3], f32),
    const_category: ConstCategory,
    size: f32,
    const_type: String
}

fn random_node_color() -> [f32; 3] {
    [0.; 3].map(|_| (random::<f32>() / 3.)*2.)
}

impl From<&NodeData> for NodePayload {
    fn from(value: &NodeData) -> Self {
        Self {
            name: value.name.clone(),
            const_category: value.const_category.clone(),
            color: random_node_color(),
            comp_color: Default::default(),
            vel: Vec2::ZERO,
            size: ((value.references.len() + 1) as f32).sqrt(),
            const_type: value.const_type.clone()
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

#[derive(Serialize, Deserialize, Clone)]
struct ForceSettings {
    r_force: f32,
    r_size: f32,
    e_force: f32,
    b_force: f32,
    stiffness: f32,
}

impl Default for ForceSettings {
    fn default() -> Self {
        Self {
            r_force: 400.,
            e_force: 0.001,
            b_force: 0.05,
            stiffness: 0.5,
            r_size: 200.
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct ColoringSettings {
    color_loss: f32,
}

impl Default for ColoringSettings {
    fn default() -> Self {
        Self { color_loss: 0.5 }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct FilterSettings {
    node_type_filter: BTreeMap<ConstCategory, bool>,
    outer_edge_cnt_filter: usize,
}

impl Default for FilterSettings {
    fn default() -> Self {
        let mut node_type_filter = BTreeMap::new();

        node_type_filter.insert(ConstCategory::Axiom, true);
        node_type_filter.insert(ConstCategory::Definition, true);
        node_type_filter.insert(ConstCategory::Theorem, true);
        node_type_filter.insert(ConstCategory::Other, false);

        Self {
            node_type_filter,
            outer_edge_cnt_filter: 10
        }
    }
}

#[derive(Serialize, Deserialize)]
struct StoredData {
    g: G,
    force_settings: ForceSettings,
    filter_settings: FilterSettings,
    coloring_settings: ColoringSettings
}

pub struct MApp {
    g: Arc<RwLock<G>>,
    g_updated: Arc<RwLock<bool>>,
    fg: G,
    last_update: Duration,
    force_settings: ForceSettings,
    filter_settings: FilterSettings,
    coloring_settings: ColoringSettings,
    data_to_load: Arc<RwLock<Option<StoredData>>>,
    fit_to_screen: Arc<RwLock<bool>>,
}

impl MApp {
    pub fn new(ctx: &CreationContext<'_>, default_file_raw: String) -> Self {
        // setup font that support math characters
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert("noto_sans_math".into(), egui::FontData::from_static(include_bytes!("../static/NotoSansMath-Regular.ttf")));
        fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "noto_sans_math".into());
        ctx.egui_ctx.set_fonts(fonts);

        let g = load_graph(default_file_raw);

        Self {
            g: Arc::new(RwLock::new(g.clone())),
            g_updated: Default::default(),
            last_update: now(),
            force_settings: Default::default(),
            fg: g,
            filter_settings: Default::default(),
            coloring_settings: Default::default(),
            data_to_load: Default::default(),
            fit_to_screen: Default::default()
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

        const SELECTED_MP: f32 = 3.;

        for &ni in &topo_sort {
            let color = self.fg.g.node_weight(ni).unwrap().payload().color;
            let size = self.fg.g[ni].payload().size;
            let size = if self.fg.g[ni].selected() {size*SELECTED_MP} else {size};
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
        let mut indices = self.fg.g.node_indices().collect::<Vec<_>>();
        if indices.len() == 0 { return };

        let neighbors = indices
            .iter()
            .map(|&ind| {
                let neigh = self.fg.g.neighbors(ind).collect::<Vec<_>>();
                (ind, neigh)
            })
            .collect::<HashMap<_, _>>();

        // Simulate edge attraction
        for &ni in &indices {
            let mut cvel = self.fg.g[ni].payload().vel;
            for &oni in &neighbors[&ni] {
                let pos = self.fg.node(ni).unwrap().location();
                let opos = self.fg.node(oni).unwrap().location();

                let dir = opos - pos;
                let dis = dir.length();
                let dir = dir.normalized();


                let eacc = self.force_settings.e_force * dis * dis;

                let mr = self.fg.g[oni].payload().mass() / self.fg.g[ni].payload().mass();

                let tot_acc = mr * eacc;

                cvel += tot_acc * dt * dir;
            }

            self.fg.node_mut(ni).unwrap().payload_mut().vel = cvel;
        }

        // Simulate repulsion
        // Create a sliding range of size RANGE_SIZE, over the nodes
        indices.sort_by(|&ni1, &ni2| self.fg.g[ni1].props().location.x.partial_cmp(&self.fg.g[ni2].props().location.x).unwrap());
        let mut bh = BinaryHeap::<Reverse<(i64, NodeIndex<u32>)>>::new();
        for &ni in &indices {
            let pos = self.fg.g[ni].location();
            while let Some(Reverse((x, oni))) = bh.pop() {
                if pos.x as i64 - x <= self.force_settings.r_size as i64 {
                    bh.push(Reverse((x, oni)));
                    break;
                }
            }

            for &Reverse((_, oni)) in &bh {
                let opos = self.fg.g[oni].location();

                let dir = opos - pos;
                let dis = dir.length();
                let dir = dir.normalized();

                if dis > self.force_settings.r_size {
                    continue;
                }

                let racc = -(self.force_settings.r_force * (self.force_settings.r_size-dis));
                let mr = self.fg.g[oni].payload().mass() / self.fg.g[ni].payload().mass();

                let racc_dt = (racc*dt);

                self.fg.g[ni].payload_mut().vel += mr * racc_dt * dir;
                self.fg.g[oni].payload_mut().vel += (1./mr) * racc_dt * (-dir);
            }

            bh.push(Reverse((pos.x as i64, ni)));
        }

        // Apply bounding force
        let mut center_of_mass = (Vec2::ZERO, 0.);

        for &ni in &indices {
            let mass = self.fg.g[ni].payload().mass();
            let loc = self.fg.g[ni].location().to_vec2();
            let tot_mass = center_of_mass.1 + mass;
            center_of_mass.0 = (center_of_mass.1 * center_of_mass.0 + mass * loc) / tot_mass;
            center_of_mass.1 = tot_mass;
        }

        let center_of_mass = center_of_mass.0;
        for &ni in &indices {
            let dir =  center_of_mass - self.fg.g[ni].location().to_vec2();
            let dis = dir.length();
            let dir = dir.normalized();

            let bacc = dis*self.force_settings.b_force;
            self.fg.g[ni].payload_mut().vel += bacc * dt * dir;
        }

        for &ni in &indices {
            let mut cvel = self.fg.g[ni].payload().vel;
            cvel = cvel * (1. - (self.force_settings.stiffness));
            const SPEED_LIMIT: f32 = 10000.;
            cvel = if (cvel.length() > SPEED_LIMIT) {cvel.normalized()*SPEED_LIMIT} else {cvel};
            let pos = self.fg.g[ni].location();
            self.fg.node_mut(ni).unwrap().payload_mut().vel = cvel;
            self.fg.node_mut(ni).unwrap().set_location(pos + cvel * dt);
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
                .with_fit_to_screen_enabled(*self.fit_to_screen.read().unwrap());
            *self.fit_to_screen.write().unwrap() = false;

            ui.add(
                &mut GraphView::new(&mut self.fg)
                    .with_styles(style_settings)
                    .with_navigations(navigations_settings)
                    .with_interactions(interaction_settings),
            );

            let g = self.g.read().unwrap();
            let node_indices = g.g.node_indices().clone().collect::<Vec<_>>();
            for ni in node_indices {
                if g.g[ni].selected() {
                    let data = g.g[ni].payload();
                    egui::Window::new(data.name.clone()).show(ctx, |ui| {
                        ui.label(data.const_type.clone());
                    });
                }
            }
        });
        egui::SidePanel::new(egui::panel::Side::Right, "Settings").show(ctx, |ui| {
            ui.collapsing("File", |ui| {
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
                if ui.button("Open extracted data").clicked() {
                    let gc = self.g.clone();
                    let guc = self.g_updated.clone();
                    let ftsc = self.fit_to_screen.clone();
                    spawn_local(async move {
                        let Some(ng_raw) = read_graph_file_dialog().await else {
                            return;
                        };
                        let ng = load_graph(ng_raw);
                        *gc.write().unwrap() = ng.clone();
                        *guc.write().unwrap() = true;
                        *ftsc.write().unwrap() = true;
                    });
                }
                if ui.button("Open stored visualization").clicked() {
                    let data_to_load = self.data_to_load.clone();
                    spawn_local(async move {
                        let Some(data_raw) = read_raw_stored_data_file_dialog().await else {
                            return;
                        };
                        let stored_data = serde_json::from_str::<StoredData>(&data_raw);
                        if let Ok(stored_data) = stored_data {
                            *data_to_load.write().unwrap() = Some(stored_data);
                        }
                        else {
                            return;
                        }
                    })
                }
                if ui.button("Save visualization").clicked() {
                    let data_to_store = serde_json::to_string(&self.save_viz()).unwrap();
                    spawn_local(async move {
                        let Some(file_handle) = AsyncFileDialog::new().add_filter("Lean Graph", &["leangraph"]).set_file_name("untitled.leangraph").save_file().await else {
                            return;
                        };
                        file_handle.write(data_to_store.as_bytes()).await.unwrap();
                    })
                }
                if ui.button("Download dependency extractor").clicked() {
                    spawn_local(async move {
                        let Some(file_handle) = AsyncFileDialog::new()
                            .set_file_name("DependencyExtractor.lean")
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

            ui.collapsing("Force simulation", |ui| {
                ui.label("Edge attraction");
                ui.add(Slider::new(
                    &mut self.force_settings.e_force,
                    (0.0)..=(0.002),
                ));
                ui.label("Repulsion force");
                ui.add(Slider::new(
                    &mut self.force_settings.r_force,
                    (10.)..=(1000.),
                ));
                ui.label("Republsion size");
                ui.add(Slider::new(
                    &mut self.force_settings.r_size,
                    (50.)..=(1000.),
                ));
                ui.label("Center bounding");
                ui.add(Slider::new(
                    &mut self.force_settings.b_force,
                    (0.)..=(0.5)
                ));
                ui.label("Stifness");
                ui.add(Slider::new(&mut self.force_settings.stiffness, (0.)..=1.));
            });
            ui.collapsing("Coloring", |ui| {
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
                    self.filter_settings.node_type_filter.get_mut(&ConstCategory::Axiom).unwrap(),
                    "Axioms",
                );
                ui.checkbox(
                    self.filter_settings.node_type_filter.get_mut(&ConstCategory::Theorem).unwrap(),
                    "Theorems",
                );
                ui.checkbox(
                    self.filter_settings.node_type_filter
                        .get_mut(&ConstCategory::Definition)
                        .unwrap(),
                    "Definitions",
                );
                ui.checkbox(
                    self.filter_settings.node_type_filter.get_mut(&ConstCategory::Other).unwrap(),
                    "Other",
                );
                ui.label("Max node out-degree");
                ui.add(Slider::new(&mut self.filter_settings.outer_edge_cnt_filter, 1..=1000));
            });

            ui.collapsing("Style", |ui| {
                let dark_mode = ui.ctx().style().visuals.dark_mode;
                if ui.button(format!("Toggle {} mode", if dark_mode {"light"} else {"dark"})).clicked() {
                    if dark_mode {
                        ui.ctx().set_visuals(Visuals::light());
                    }
                    else {
                        ui.ctx().set_visuals(Visuals::dark());
                    }
                }
                if ui.button("Fit to screen").clicked() {
                    *self.fit_to_screen.write().unwrap() = true;
                }
            });


            ui.allocate_space(ui.available_size()-Vec2::Y*30.);

            ui.horizontal(|ui| {
                ui.label("Contact:");
                ui.add(Hyperlink::from_label_and_url("GitHub", "https://github.com/patrik-cihal/lean-graph"));
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
                if self.filter_settings.node_type_filter[&node.payload().const_category]
                    && g.g.neighbors(ni).count() <= self.filter_settings.outer_edge_cnt_filter
                {
                    Some(node.clone())
                } else {
                    None
                }
            },
            |_, edge| Some(edge.clone()),
        ));
    }
    fn save_viz(&self) -> StoredData {
        StoredData {
            filter_settings: self.filter_settings.clone(),
            force_settings: self.force_settings.clone(),
            g: self.g.read().unwrap().clone(),
            coloring_settings: self.coloring_settings.clone(),
        }
    }
    fn load_stored_data(&mut self, data: StoredData) {
        *self.g.write().unwrap() = data.g;
        *self.g_updated.write().unwrap() = true;
        self.last_update = now();
        self.force_settings = data.force_settings;
        self.filter_settings = data.filter_settings;
        self.coloring_settings = data.coloring_settings;
        *self.fit_to_screen.write().unwrap() = true;
    }
}

impl App for MApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        let mut data_to_load_write = self.data_to_load.write().unwrap();
        if let Some(data_to_load) = data_to_load_write.take() {
            drop(data_to_load_write);
            self.load_stored_data(data_to_load);
        }
        else {
            drop(data_to_load_write);
        }
        self.update_filter_graph();
        let ct = now();
        let dt = (ct.clone() - self.last_update).as_secs_f32();
        self.simulate_force_graph(dt.min(0.032));
        self.last_update = ct;
        self.color_nodes();
        self.draw_ui(ctx);
    }
}

fn load_graph(default_file_raw: String) -> G {
    let nodes = serde_json::from_str::<Vec<NodeData>>(&default_file_raw).unwrap();
    let mut sg = StableGraph::<_, _, Directed, _>::default();

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
    let rnd_angle = random::<f32>()*2.*PI;
    let rnd_dist = random::<f32>().sqrt()*size;
    let pos =  Pos2::new(rnd_angle.cos(), rnd_angle.sin()) * rnd_dist;
    pos
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

pub async fn read_raw_stored_data_file_dialog() -> Option<String> {
    let Some(file_handle) = AsyncFileDialog::new()
        .add_filter("Lean Graph", &["leangraph"])
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
    let resp = reqwest::get(format!("{SERVER_ADDR}/static/DependencyExtractor.lean")).await?;
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
