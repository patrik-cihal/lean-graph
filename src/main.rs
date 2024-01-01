use lean_dependency_graph::MApp;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    let data_raw = std::fs::read_to_string("static/Nat.zero_add.json").unwrap();
    eframe::run_native(
        "lean graph",
        native_options,
        Box::new(|cc| Box::new(MApp::new(cc, data_raw))),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:

    use lean_dependency_graph::read_graph_url;
    use lean_dependency_graph::SERVER_ADDR;

    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        // let data_raw = read_graph_file_dialog().await;
        let data_raw = read_graph_url(&format!("{}/static/Nat.zero_add.json", SERVER_ADDR))
            .await
            .unwrap();
        eframe::WebRunner::new()
            .start(
                "lean-graph-canvas", // hardcode it
                web_options,
                Box::new(|cc| Box::new(MApp::new(cc, data_raw))),
            )
            .await
            .expect("failed to start eframe");
    });
}
 