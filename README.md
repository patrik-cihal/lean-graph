# Lean Graph

Interactive visualization of dependencies for any theorem/definitions in your Lean project.


![Fermat last theorem four](fermat-last-theorem-4-example.png)

## How to use

### Run in your browser: [lean-graph.com](https://lean-graph.com/)

### Run locally
1. Get [Rustup](https://rustup.rs/)
2. Install `Lean Graph` by executing this command in your terminal: ```cargo install --git "https://github.com/patrik-cihal/lean-graph"```
3. Run `Lean Graph` from the terminmal under the name `lean-graph`

### Extracting graph from `.lean` files
1. Copy the `DependencyExtractor.lean` into your project folder (either from GitHub, or download it in the web app)
2. In the top of the file import the files where are the theorems/definitions you want to extract the graph for
3. In the bottom of the file there is an #eval line where you can specify your own custom theorem/definition name
4. Uncomment that same line to get the `.json` file

## What's next

### Additional features
- After clicking on node, allow for seeing the documentation
- Lazy loading of depending nodes
- Option to visualize any Mathlib constant, without the need of running script locally

If you want to see these features, any contribution is welcome.
