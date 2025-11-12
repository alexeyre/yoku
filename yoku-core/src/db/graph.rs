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
        // We MERGE both exercises (variant may not exist yet; create lightweight placeholder)
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

    /// Seed the graph with a small set of exercises/muscles/equipment for testing.
    ///
    /// This is idempotent: it uses MERGE for all nodes/relationships so it can be
    /// safely re-run during development.
    pub async fn seed_defaults(&self) -> Result<()> {
        // Hard-coded seed data that's useful for initial testing.
        // We create a few muscles, exercises, and equipment, and link them.
        let now = Utc::now().to_rfc3339();

        // Create muscles
        let muscles = vec![
            "lateral_deltoid",
            "anterior_deltoid",
            "triceps",
            "pectoralis_major",
        ];
        for m in muscles {
            let q = query(
                "MERGE (mm:Muscle { name: $name }) \
                 ON CREATE SET mm.created_at = $now, mm.updated_at = $now \
                 ON MATCH SET mm.updated_at = $now RETURN mm.name AS name",
            )
            .param("name", m.to_string())
            .param("now", now.clone());
            let mut res = self.graph.execute(q).await?;
            while let Ok(Some(_)) = res.next().await {}
        }

        // Create equipment
        let equipments = vec!["dumbbell", "barbell", "cable_machine", "bench"];
        for eq in equipments {
            let q = query(
                "MERGE (e:Equipment { name: $name }) \
                 ON CREATE SET e.created_at = $now, e.updated_at = $now \
                 ON MATCH SET e.updated_at = $now RETURN e.name AS name",
            )
            .param("name", eq.to_string())
            .param("now", now.clone());
            let mut res = self.graph.execute(q).await?;
            while let Ok(Some(_)) = res.next().await {}
        }

        // Create a few exercises (slug, name, description)
        let exercises = vec![
            (
                "bench-press",
                "Bench Press",
                "Barbell bench press - pressing movement primarily for chest",
            ),
            (
                "incline-bench-press",
                "Incline Bench Press",
                "Incline bench press - upper chest emphasis",
            ),
            (
                "dumbbell-lateral-raise",
                "Dumbbell Lateral Raise",
                "Shoulder abduction targeting lateral deltoid",
            ),
            (
                "cable-lateral-raise",
                "Cable Lateral Raise",
                "Standing cable lateral raise with constant tension",
            ),
            (
                "cable-tricep-extension",
                "Cable Tricep Extension",
                "Cable-based triceps extension",
            ),
        ];

        for (slug, name, desc) in exercises {
            let q = query(
                "MERGE (ex:Exercise { slug: $slug }) \
                 ON CREATE SET ex.name = $name, ex.description = $desc, ex.created_at = $now, ex.updated_at = $now \
                 ON MATCH SET ex.name = $name, ex.description = $desc, ex.updated_at = $now RETURN ex.slug AS slug",
            )
            .param("slug", slug.to_string())
            .param("name", name.to_string())
            .param("desc", desc.to_string())
            .param("now", now.clone());
            let mut res = self.graph.execute(q).await?;
            while let Ok(Some(_)) = res.next().await {}
        }

        // Link exercises -> muscles (simple relation_type strings for now)
        let mappings = vec![
            ("dumbbell-lateral-raise", "lateral_deltoid", "primary"),
            ("cable-lateral-raise", "lateral_deltoid", "primary"),
            ("bench-press", "pectoralis_major", "primary"),
            ("bench-press", "triceps", "secondary"),
            ("incline-bench-press", "pectoralis_major", "primary"),
            ("incline-bench-press", "anterior_deltoid", "secondary"),
            ("cable-tricep-extension", "triceps", "primary"),
        ];

        for (ex_slug, muscle_name, rel) in mappings {
            let q = query(
                "MATCH (ex:Exercise { slug: $slug }), (m:Muscle { name: $mname }) \
                 MERGE (ex)-[r:WORKS_MUSCLE]->(m) \
                 ON CREATE SET r.relation_type = $rel, r.created_at = $now, r.updated_at = $now \
                 ON MATCH SET r.relation_type = $rel, r.updated_at = $now RETURN ex.slug AS slug, m.name AS muscle",
            )
            .param("slug", ex_slug.to_string())
            .param("mname", muscle_name.to_string())
            .param("rel", rel.to_string())
            .param("now", now.clone());
            let mut res = self.graph.execute(q).await?;
            while let Ok(Some(_)) = res.next().await {}
        }

        // Link exercises -> equipment
        let ex_eq = vec![
            ("dumbbell-lateral-raise", "dumbbell", 1.0_f32),
            ("cable-lateral-raise", "cable_machine", 1.0_f32),
            ("bench-press", "barbell", 1.0_f32),
            ("bench-press", "bench", 1.0_f32),
            ("incline-bench-press", "barbell", 0.9_f32),
            ("cable-tricep-extension", "cable_machine", 1.0_f32),
        ];

        for (ex_slug, eq_name, conf) in ex_eq {
            let q = query(
                "MATCH (ex:Exercise { slug: $slug }), (eq:Equipment { name: $eqname }) \
                 MERGE (ex)-[r:USES_EQUIPMENT]->(eq) \
                 ON CREATE SET r.confidence = $conf, r.created_at = $now, r.updated_at = $now \
                 ON MATCH SET r.confidence = $conf, r.updated_at = $now RETURN ex.slug AS slug, eq.name AS equipment",
            )
            .param("slug", ex_slug.to_string())
            .param("eqname", eq_name.to_string())
            .param("conf", conf)
            .param("now", now.clone());
            let mut res = self.graph.execute(q).await?;
            while let Ok(Some(_)) = res.next().await {}
        }

        // A couple of variation links (bidirectional could be created from both directions if desired)
        let variations = vec![
            (
                "cable-lateral-raise",
                "dumbbell-lateral-raise",
                0.85_f32,
                "minor",
            ),
            ("bench-press", "incline-bench-press", 0.65_f32, "major"),
        ];

        for (a, b, overlap, rel) in variations {
            let q = query(
                "MATCH (a:Exercise { slug: $a }), (b:Exercise { slug: $b }) \
                 MERGE (a)-[r:VARIATION_OF]->(b) \
                 ON CREATE SET r.overlap = $overlap, r.relation_type = $rel, r.created_at = $now, r.updated_at = $now \
                 ON MATCH SET r.overlap = $overlap, r.relation_type = $rel, r.updated_at = $now RETURN a.slug AS a, b.slug AS b",
            )
            .param("a", a.to_string())
            .param("b", b.to_string())
            .param("overlap", overlap)
            .param("rel", rel.to_string())
            .param("now", now.clone());
            let mut res = self.graph.execute(q).await?;
            while let Ok(Some(_)) = res.next().await {}
        }

        Ok(())
    }

    /// Dump a simple textual representation of the graph for debugging.
    /// This fetches exercises and their WORKS_MUSCLE relationships and prints them.
    pub async fn dump_graph(&self, limit: i64) -> Result<()> {
        // Query exercises and linked muscles (limit controls number of rows returned)
        let q = query(
            "MATCH (e:Exercise)-[r:WORKS_MUSCLE]->(m:Muscle) \
             RETURN e.slug AS slug, e.name AS name, r.relation_type AS relation_type, m.name AS muscle \
             LIMIT $limit",
        )
        .param("limit", limit);

        let mut result = self.graph.execute(q).await?;

        while let Ok(Some(row)) = result.next().await {
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

        Ok(())
    }
}
