use anyhow::Result;
use neo4rs::*;
use std::{sync::Arc, time::Instant};
use step_merger::step::StepEntry;

use log::info;

pub struct Neo4J {
    graph: Arc<Graph>,
}

pub struct Neo4JOptions {
    /// The URI of the Neo4J database, e.g., neo4j://127.0.0.1:7687.
    pub uri: String,

    /// The user name for the Neo4J database.
    pub user: String,

    /// The password for the Neo4J database.
    pub pass: String,
}

impl Neo4J {
    /// Creates a new Neo4J database connection.
    ///
    /// # Arguments
    /// * `options` - The options for the Neo4J database connection.
    pub async fn new(options: Neo4JOptions) -> Result<Neo4J> {
        info!("Connect to Neo4J {} database...", options.uri);
        let graph = Arc::new(Graph::new(options.uri, options.user, options.pass).await?);
        info!("Connect to Neo4J database...DONE");

        Self::initialize_schema(graph.clone()).await?;

        Ok(Self { graph })
    }

    pub async fn insert_step_entries(&self, entries: &[StepEntry]) -> anyhow::Result<()> {
        info!("Insert step entries...");
        let start = Instant::now();
        let graph = self.graph.clone();
        let mut tx = graph.start_txn().await?;
        for entry in entries.iter() {
            tx.run(
                query("CREATE (e:Entry {id: $id, definition: $definition})")
                    .param("id", entry.get_id() as i64)
                    .param("definition", entry.get_definition()),
            )
            .await?;
        }
        tx.commit().await?;
        info!(
            "Insert step entries...DONE in {} s",
            start.elapsed().as_secs_f32()
        );

        info!("Insert entry references...");
        let start = Instant::now();
        let mut tx = graph.start_txn().await?;
        for entry in entries.iter() {
            let r0 = entry.get_id() as i64;

            for r1 in entry.get_references().iter().cloned().map(|r| r as i64) {
                tx.run(
                    query(
                        "MATCH (r0:Entry {id: $r0}), (r1:Entry {id: $r1}) CREATE (r0)-[:REF]->(r1)",
                    )
                    .param("r0", r0)
                    .param("r1", r1),
                )
                .await?;
            }
        }
        tx.commit().await?;
        info!(
            "Insert entry references...DONE in {} s",
            start.elapsed().as_secs_f32()
        );

        Ok(())
    }

    /// Initialize the database schema.
    ///
    /// # Arguments
    /// `graph` - The graph DB instance to use for initialization.
    async fn initialize_schema(graph: Arc<Graph>) -> anyhow::Result<()> {
        info!("Initialize database...");
        graph
            .run(query(
                "CREATE INDEX entry_id_index IF NOT EXISTS FOR (e:Entry) ON e.id;",
            ))
            .await?;

        info!("Initialize database...DONE");
        Ok(())
    }
}
