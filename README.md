# Lean Graph

Visualization of dependencies for any theorem/definitions in your Lean project.

![](example-pic.png)

## How to use

1. Copy the get_graph_meta.lean into your project folder
2. In the top of the file import the files where are the theorems/definitions you want to extract the graph for
3. In the bottom of the file there is an #eval line where you can specify your own custom theorem/definition name
4. Uncomment that same line to get the .json file
5. Run the Rust project using `cargo run --release` or `cargo r` and select your .json file