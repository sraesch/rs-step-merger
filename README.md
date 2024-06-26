# Step Merger
The step merger consumes a JSON file that defines an assembly structure and generates a single STEP file that contains all referenced STEP files and merges them into a single monolithic STEP file.

For example, the following JSON file defines an assembly structure that contains two instances of the same cube. The first cube is translated to the left by 2 units, and the second cube is translated to the right by 2 units. The root node contains metadata that can be used to store additional information about the assembly structure.
```json
{
  "nodes": [
    {
      "label": "Root Node",
      "children": [1, 2],
      "metadata": [
        {
          "key": "key1",
          "value": "value1"
        },
        {
          "key": "key2",
          "value": "value2"
        }
      ]
    },
    {
      "label": "Left Cube Node",
      "transform": [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, -2, 0, 0, 1],
      "link": "cube.stp"
    },
    {
      "label": "Right Cube Node",
      "transform": [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 2, 0, 0, 1],
      "link": "cube.stp"
    }
  ]
}
```

## Build project
First, you'll need to have rust installed. See [here](https://www.rust-lang.org/tools/install) for instructions on how to install rust.
After you have rust installed, you can build the project by running the following command:
```bash
cargo build
```
and for the release build:
```bash
cargo build --release
```

## Run project
After you have built the project, you can find the executable in the `target/debug` directory or `target/release` directory for release. In order to do a small test run, you can run the following command:
```bash
cd test_data
../target/debug/step-merger-cli -i one-cube.json -o gen-cube.stp
```
This will consume the assembly structure being defined in `one-cube.json` and generate a STEP file `gen-cube.stp` in the same directory by loading all referenced files and merging them into a single monolithic STEP file.
