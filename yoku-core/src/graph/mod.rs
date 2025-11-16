use std::env;
use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use neo4rs::{Graph, query};
use tokio::sync::OnceCell;

use log::{debug, error, info, warn};

use crate::db::models::{Exercise, Muscle};

pub struct GraphManager {
    graph: Arc<Graph>,
}

static GRAPH_CLIENT: OnceCell<Arc<Graph>> = OnceCell::const_new();

impl GraphManager {
    pub async fn connect() -> Result<Self> {
        debug!("GraphManager::connect called");
        let graph = GraphManager::get_graph().await?;
        info!("GraphManager connected to Neo4j");
        Ok(Self { graph })
    }

    async fn get_graph() -> Result<Arc<Graph>> {
        debug!("GraphManager::get_graph initializing or returning cached client");
        let arc_ref = GRAPH_CLIENT
            .get_or_try_init(|| async {
                let host = env::var("NEO4J_HOST").expect("NEO4J_HOST environment variable not set");
                let user = env::var("NEO4J_USER").expect("NEO4J_USER environment variable not set");
                // intentionally do NOT log the password
                let _password = env::var("NEO4J_PASSWORD")
                    .expect("NEO4J_PASSWORD environment variable not set");

                debug!("creating new Graph client for host={} user={}", host, user);
                let g = Graph::new(&host, &user, &_password).await?;
                info!("created new Graph client for host={}", host);
                Ok::<Arc<Graph>, anyhow::Error>(Arc::new(g))
            })
            .await?;

        Ok(arc_ref.clone())
    }

    pub async fn upsert_muscle(&self, muscle: &Muscle) -> Result<()> {
        debug!("upsert_muscle called name={}", muscle.name);
        let now = Utc::now().to_rfc3339();
        let q = query(
            "MERGE (m:Muscle { name: $name }) \
             ON CREATE SET m.created_at = $now, m.updated_at = $now \
             ON MATCH SET m.updated_at = $now \
             RETURN m.name AS name",
        )
        .param("name", muscle.name.clone())
        .param("now", now);

        let mut result = self.graph.execute(q).await.map_err(|e| {
            error!("upsert_muscle execute failed for {}: {}", muscle.name, e);
            anyhow::Error::from(e)
        })?;

        while let Ok(Some(_row)) = result.next().await {}
        info!("upsert_muscle completed name={}", muscle.name);
        Ok(())
    }

    pub async fn upsert_exercise(&self, exercise: &Exercise) -> Result<()> {
        debug!(
            "upsert_exercise called slug={} name={}",
            exercise.slug, exercise.name
        );
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

        let mut result = self.graph.execute(q).await.map_err(|e| {
            error!(
                "upsert_exercise execute failed for {}: {}",
                exercise.slug, e
            );
            anyhow::Error::from(e)
        })?;
        while let Ok(Some(_row)) = result.next().await {}
        info!(
            "upsert_exercise completed slug={} name={}",
            exercise.slug, exercise.name
        );
        Ok(())
    }

    pub async fn upsert_exercise_muscle(
        &self,
        exercise: &Exercise,
        muscle: &Muscle,
        relation_type: &str,
    ) -> Result<()> {
        debug!(
            "upsert_exercise_muscle called exercise={} muscle={} relation_type={}",
            exercise.slug, muscle.name, relation_type
        );
        let now = Utc::now().to_rfc3339();

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

        let mut result = self.graph.execute(q).await.map_err(|e| {
            error!(
                "upsert_exercise_muscle execute failed for exercise={} muscle={}: {}",
                exercise.slug, muscle.name, e
            );
            anyhow::Error::from(e)
        })?;
        while let Ok(Some(_row)) = result.next().await {}
        info!(
            "upsert_exercise_muscle completed exercise={} muscle={}",
            exercise.slug, muscle.name
        );
        Ok(())
    }

    pub async fn upsert_equipment_and_link(
        &self,
        exercise: &Exercise,
        equipment_name: &str,
        confidence: f32,
    ) -> Result<()> {
        debug!(
            "upsert_equipment_and_link called exercise={} equipment={} confidence={}",
            exercise.slug, equipment_name, confidence
        );
        let now = Utc::now().to_rfc3339();
        let q = query(
            "MERGE (e:Exercise { slug: $slug }) \
             ON CREATE SET e.name = $ename, e.description = $description, e.created_at = $now, e.updated_at = $now \
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

        let mut result = self.graph.execute(q).await.map_err(|e| {
            error!(
                "upsert_equipment_and_link execute failed for exercise={} equipment={}: {}",
                exercise.slug, equipment_name, e
            );
            anyhow::Error::from(e)
        })?;
        while let Ok(Some(_row)) = result.next().await {}
        info!(
            "upsert_equipment_and_link completed exercise={} equipment={}",
            exercise.slug, equipment_name
        );
        Ok(())
    }

    pub async fn upsert_variation(
        &self,
        exercise: &Exercise,
        variant_slug: &str,
        overlap: f32,
        relation_type: &str,
    ) -> Result<()> {
        debug!(
            "upsert_variation called exercise={} variant={} overlap={} relation_type={}",
            exercise.slug, variant_slug, overlap, relation_type
        );
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

        let mut result = self.graph.execute(q).await.map_err(|e| {
            error!(
                "upsert_variation execute failed for exercise={} variant={}: {}",
                exercise.slug, variant_slug, e
            );
            anyhow::Error::from(e)
        })?;
        while let Ok(Some(_row)) = result.next().await {}
        info!(
            "upsert_variation completed exercise={} variant={}",
            exercise.slug, variant_slug
        );
        Ok(())
    }

    pub async fn dump_graph(&self, limit: i64) -> Result<()> {
        debug!("dump_graph called with limit={}", limit);
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

        let mut result_exercise_muscles =
            self.graph.execute(q_exercise_muscles).await.map_err(|e| {
                error!("dump_graph failed executing exercise_muscles query: {}", e);
                anyhow::Error::from(e)
            })?;
        let mut result_exercise_equipment = self
            .graph
            .execute(q_exercise_equipment)
            .await
            .map_err(|e| {
                error!(
                    "dump_graph failed executing exercise_equipment query: {}",
                    e
                );
                anyhow::Error::from(e)
            })?;

        while let Ok(Some(row)) = result_exercise_muscles.next().await {
            let slug: String = row.get("slug").unwrap_or_default();
            let name: String = row.get("name").unwrap_or_default();
            let relation_type: String = row.get("relation_type").unwrap_or_default();
            let muscle: String = row.get("muscle").unwrap_or_default();
            info!(
                "{} ({}) -[WORKS_MUSCLE: {}]-> {}",
                name, slug, relation_type, muscle
            );
        }

        while let Ok(Some(row)) = result_exercise_equipment.next().await {
            let slug: String = row.get("slug").unwrap_or_default();
            let name: String = row.get("name").unwrap_or_default();
            let relation_type: String = row.get("relation_type").unwrap_or_default();
            let equipment: String = row.get("equipment").unwrap_or_default();
            info!(
                "{} ({}) -[USES_EQUIPMENT: {}]-> {}",
                name, slug, relation_type, equipment
            );
        }

        Ok(())
    }
}
