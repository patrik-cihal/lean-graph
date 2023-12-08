
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    use lean_dependency_graph::MApp;


    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "lean graph",
        native_options,
        Box::new(|cc| Box::new(MApp::new(cc))),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    use optimality_solver::MApp;

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "optimality-solver-canvas", // hardcode it
                web_options,
                Box::new(|cc| Box::new(MApp::new(cc))),
            )
            .await
            .expect("failed to start eframe");
    });
}