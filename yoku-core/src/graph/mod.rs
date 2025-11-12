use std::env;
use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use neo4rs::{Graph, query};
use tokio::sync::OnceCell;

use crate::db::models::{Exercise, Muscle};

pub struct GraphManager {
    graph: Arc<Graph>,
}

static GRAPH_CLIENT: OnceCell<Arc<Graph>> = OnceCell::const_new();

impl GraphManager {
    /// Connect to the Neo4j instance using env vars and return a manager.
    pub async fn connect() -> Result<Self> {
        let graph = GraphManager::get_graph().await?;
        Ok(Self { graph })
    }

    /// Internal helper: initialize a shared neo4j client once.
    async fn get_graph() -> Result<Arc<Graph>> {
        let arc_ref = GRAPH_CLIENT
            .get_or_try_init(|| async {
                let host = env::var("NEO4J_HOST").expect("NEO4J_HOST environment variable not set");
                let user = env::var("NEO4J_USER").expect("NEO4J_USER environment variable not set");
                let password = env::var("NEO4J_PASSWORD")
                    .expect("NEO4J_PASSWORD environment variable not set");

                let g = Graph::new(&host, &user, &password).await?;
                Ok::<Arc<Graph>, anyhow::Error>(Arc::new(g))
            })
            .await?;

        Ok(arc_ref.clone())
    }

    /// Upsert a muscle node. Uses `name` as the unique key.
    pub async fn upsert_muscle(&self, muscle: &Muscle) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let q = query(
            "MERGE (m:Muscle { name: $name }) \
             ON CREATE SET m.created_at = $now, m.updated_at = $now \
             ON MATCH SET m.updated_at = $now \
             RETURN m.name AS name",
        )
        .param("name", muscle.name.clone())
        .param("now", now);

        let mut result = self.graph.execute(q).await?;
        // consume result once to ensure query finishes
        while let Ok(Some(_row)) = result.next().await {}
        Ok(())
    }

    /// Upsert an exercise node. Uses `slug` as the unique key and stores some basic properties.
    pub async fn upsert_exercise(&self, exercise: &Exercise) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let description = exercise.description.clone().unwrap_or_default();
        let q = query(
            "MERGE (e:Exercise { slug: $slug }) \
             ON CREATE SET e.name = $name, e.description = $description, e.created_at = $now, e.updated_at = $now \
             ON MATCH SET e.name = $name, e.description = $description, e.updated_at = $now \
             RETURN e.slug AS slug",
        )
        .param("slug", exercise.slug.clone())
        .param("name", exercise.name.clone())
        .param("description", description)
        .param("now", now);

        let mut result = self.graph.execute(q).await?;
        while let Ok(Some(_row)) = result.next().await {}
        Ok(())
    }

    /// Create/refresh a relationship between an exercise and a muscle.
    /// `relation_type` is a free-form string stored on the relationship (e.g., "primary", "secondary").
    pub async fn upsert_exercise_muscle(
        &self,
        exercise: &Exercise,
        muscle: &Muscle,
        relation_type: &str,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        // Ensure nodes exist (idempotent MERGE), then MERGE the relationship with properties.
        let q = query(
            "MERGE (e:Exercise { slug: $slug }) \
             ON CREATE SET e.name = $ename, e.created_at = $now, e.updated_at = $now \
             ON MATCH SET e.updated_at = $now \
             MERGE (m:Muscle { name: $mname }) \
             ON CREATE SET m.created_at = $now, m.updated_at = $now \
             ON MATCH SET m.updated_at = $now \
             MERGE (e)-[r:WORKS_MUSCLE]->(m) \
             ON CREATE SET r.relation_type = $relation_type, r.created_at = $now, r.updated_at = $now \
             ON MATCH SET r.relation_type = $relation_type, r.updated_at = $now \
             RETURN e.slug AS slug, m.name AS muscle",
        )
        .param("slug", exercise.slug.clone())
        .param("ename", exercise.name.clone())
        .param("mname", muscle.name.clone())
        .param("relation_type", relation_type.to_string())
        .param("now", now);

        let mut result = self.graph.execute(q).await?;
        while let Ok(Some(_row)) = result.next().await {}
        Ok(())
    }

    /// Upsert an equipment node and create a relation from exercise -> equipment.
    /// Equipment is only in the graph; it's identified by name.
    pub async fn upsert_equipment_and_link(
        &self,
        exercise: &Exercise,
        equipment_name: &str,
        confidence: f32,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let q = query(
            "MERGE (e:Exercise { slug: $slug }) \
             ON CREATE SET e.name = $ename, e.created_at = $now, e.updated_at = $now \
             ON MATCH SET e.updated_at = $now \
             MERGE (eq:Equipment { name: $eqname }) \
             ON CREATE SET eq.created_at = $now, eq.updated_at = $now \
             ON MATCH SET eq.updated_at = $now \
             MERGE (e)-[r:USES_EQUIPMENT]->(eq) \
             ON CREATE SET r.confidence = $confidence, r.created_at = $now, r.updated_at = $now \
             ON MATCH SET r.confidence = $confidence, r.updated_at = $now \
             RETURN e.slug AS slug, eq.name AS equipment",
        )
        .param("slug", exercise.slug.clone())
        .param("ename", exercise.name.clone())
        .param("eqname", equipment_name.to_string())
        .param("confidence", confidence)
        .param("now", now);

        let mut result = self.graph.execute(q).await?;
        while let Ok(Some(_row)) = result.next().await {}
        Ok(())
    }

    /// Upsert a variation relationship between two exercises.
    /// `overlap` should be 0.0..1.0; `relation_type` e.g. "minor", "major", "alternative".
    pub async fn upsert_variation(
        &self,
        exercise: &Exercise,
        variant_slug: &str,
        overlap: f32,
        relation_type: &str,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let q = query(
            "MERGE (a:Exercise { slug: $slug }) \
             ON CREATE SET a.created_at = $now, a.updated_at = $now \
             ON MATCH SET a.updated_at = $now \
             MERGE (b:Exercise { slug: $vslug }) \
             ON CREATE SET b.created_at = $now, b.updated_at = $now \
             ON MATCH SET b.updated_at = $now \
             MERGE (a)-[r:VARIATION_OF]->(b) \
             ON CREATE SET r.overlap = $overlap, r.relation_type = $relation_type, r.created_at = $now, r.updated_at = $now \
             ON MATCH SET r.overlap = $overlap, r.relation_type = $relation_type, r.updated_at = $now \
             RETURN a.slug AS a, b.slug AS b",
        )
        .param("slug", exercise.slug.clone())
        .param("vslug", variant_slug.to_string())
        .param("overlap", overlap)
        .param("relation_type", relation_type.to_string())
        .param("now", now);

        let mut result = self.graph.execute(q).await?;
        while let Ok(Some(_row)) = result.next().await {}
        Ok(())
    }
    /// Dump a simple textual representation of the graph for debugging.
    /// This fetches exercises and their WORKS_MUSCLE relationships and prints them.
    pub async fn dump_graph(&self, limit: i64) -> Result<()> {
        // Query exercises and linked muscles (limit controls number of rows returned)
        let q_exercise_muscles = query(
            "MATCH (e:Exercise)-[r:WORKS_MUSCLE]->(m:Muscle) \
             RETURN e.slug AS slug, e.name AS name, r.relation_type AS relation_type, m.name AS muscle \
             LIMIT $limit",
        )
        .param("limit", limit);

        let q_exercise_equipment = query(
            "MATCH (e:Exercise)-[r:USES_EQUIPMENT]->(eq:Equipment) \
             RETURN e.slug AS slug, e.name AS name, r.relation_type AS relation_type, eq.name AS equipment \
             LIMIT $limit",
        )
        .param("limit", limit);

        let mut result_exercise_muscles = self.graph.execute(q_exercise_muscles).await?;
        let mut result_exercise_equipment = self.graph.execute(q_exercise_equipment).await?;

        while let Ok(Some(row)) = result_exercise_muscles.next().await {
            // row.get returns Option<T> where T implements FromValue
            let slug: String = row.get("slug").unwrap_or_default();
            let name: String = row.get("name").unwrap_or_default();
            let relation_type: String = row.get("relation_type").unwrap_or_default();
            let muscle: String = row.get("muscle").unwrap_or_default();
            println!(
                "{} ({}) -[WORKS_MUSCLE: {}]-> {}",
                name, slug, relation_type, muscle
            );
        }

        while let Ok(Some(row)) = result_exercise_equipment.next().await {
            // row.get returns Option<T> where T implements FromValue
            let slug: String = row.get("slug").unwrap_or_default();
            let name: String = row.get("name").unwrap_or_default();
            let relation_type: String = row.get("relation_type").unwrap_or_default();
            let equipment: String = row.get("equipment").unwrap_or_default();
            println!(
                "{} ({}) -[USES_EQUIPMENT: {}]-> {}",
                name, slug, relation_type, equipment
            );
        }

        Ok(())
    }
}
