# Step Merger

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

## Other things

### Debugging in Neo4J
In order to visualize the elements and their relations inside a single STEP file, I've created a debugging scenario that uses Neo4J to visualize the elements and their relations.
In order to run the debugging scenario for Neo4J, first start the Neo4J server by running the `run_neo4j.sh` script. For that you'll require docker.
Afterwards, you can feed the STEP file into the Neo4J database by running the following command:
```bash
./target/debug/step-export-neo4j -i test_data/cube.stp -u neo4j -p 1234and5
```
It will load the STEP file `cube.stp` and dump the elements and their relations into the Neo4J database.

## Reverse engineered STEP specification
Some step functions being explained [STEP Specification](./doc/step-spec.md)
