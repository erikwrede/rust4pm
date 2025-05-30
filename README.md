<div align="center">
<h1>OCGRAPHCONF: Object-Centric Graph-Based Conformance checking</h1>
</div>

This repository is a mono-repo based on a fork of rust4pm by @aarkue and will be integrated into the library in the future.

# Install guide
- Make sure all dependencies for SCIP are installed: https://www.scipopt.org/doc/html/INSTALL.php
- Chose a ModelStateInterface from oc_state_space::impl - either OCPN or OCPT, or create your own!
- Modify cli/src/main.rs to use your ModelStateInterface
- Run the following commands to build the cli & start the eval

```sh
cd cli
cargo build --release

cd ..

mkdir cli-output

./target/release/cli controller \
    --json-folder ./test_data/varsbpi \
    --petri-net ./test_data/oc_petri_net.json \
    --smallest-case ./test_data/shortest_case_graph.json \
    --output-dir ./cli-output
```
- let it run for a while, check on the progress. It will create a lot of files in the `cli-output` folder

Please contact the authors for a test dataset.