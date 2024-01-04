# Lean Graph

Interactive visualization of dependencies for any theorem/definitions in your Lean project.


![Fermat last theorem four](fermat-last-theorem-4-example.png)

## How to use

### In your browser: [lean-graph.com](https://lean-graph.com/)

### Or run locally
1. Copy the `DependencyExtractor.lean` into your project folder (either from GitHub, or download it in the web app)
2. In the top of the file import the files where are the theorems/definitions you want to extract the graph for
3. In the bottom of the file there is an #eval line where you can specify your own custom theorem/definition name
4. Uncomment that same line to get the .json file
5. Run the Rust project using `cargo run --release` or `cargo r` and select your .json file

## What's next

### Additional features
- After clicking on node, allow for seeing the documentation
- Lazy loading of depending nodes
- Option to visualize any Mathlib constant, without the need of running script locally

If you want to see these features, any contribution is welcome.
