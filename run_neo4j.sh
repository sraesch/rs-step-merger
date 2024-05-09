#!/bin/bash
docker run \
    --publish=7474:7474 --publish=7687:7687 \
    --volume=${PWD}/neo4j/data:/data \
    -e NEO4J_AUTH='neo4j/1234and5' \
    -e NEO4J_apoc_export_file_enabled=true \
    -e NEO4J_apoc_import_file_enabled=true \
    -e NEO4J_apoc_import_file_use__neo4j__config=true \
    -e NEO4J_PLUGINS=\[\"apoc\"\] \
    neo4j:5.11