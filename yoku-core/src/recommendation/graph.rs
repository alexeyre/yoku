use crate::db::models as dbm;
use crate::db::models::ExercisePatternType;
use crate::db::operations::slugify;
use anyhow::{Result, anyhow};
use indradb::QueryExt;
use indradb::{Database, Datastore, MemoryDatastore, QueryOutputValue, RocksdbDatastore, ijson};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MuscleUsageType {
    Primary,
    Synergist,
    Stabilizer,
}

impl MuscleUsageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MuscleUsageType::Primary => "primary",
            MuscleUsageType::Synergist => "synergist",
            MuscleUsageType::Stabilizer => "stabilizer",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "primary" => Ok(MuscleUsageType::Primary),
            "synergist" => Ok(MuscleUsageType::Synergist),
            "stabilizer" => Ok(MuscleUsageType::Stabilizer),
            _ => Err(anyhow!("Invalid muscle usage type: {}", s)),
        }
    }

    pub fn transfer_weight(&self) -> f64 {
        match self {
            MuscleUsageType::Primary => 1.0,
            MuscleUsageType::Synergist => 0.5,
            MuscleUsageType::Stabilizer => 0.2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuscleInvolvement {
    pub scale_factor: f64,
    pub usage_type: MuscleUsageType,
}

impl MuscleInvolvement {
    pub fn new(scale_factor: f64, usage_type: MuscleUsageType) -> Self {
        Self {
            scale_factor: scale_factor.clamp(0.0, 1.0),
            usage_type,
        }
    }

    pub fn effective_weight(&self) -> f64 {
        self.scale_factor * self.usage_type.transfer_weight()
    }
}

pub struct GraphManager<T: Datastore> {
    db: Database<T>,
}

impl GraphManager<RocksdbDatastore> {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let db = RocksdbDatastore::new_db(db_path)?;
        let mut gm = Self { db };
        gm.setup_indexed_properties()?;
        Ok(gm)
    }
}

impl GraphManager<MemoryDatastore> {
    pub fn new() -> Result<Self> {
        let db = MemoryDatastore::new_db();
        let mut gm = Self { db };
        gm.setup_indexed_properties()?;
        Ok(gm)
    }
}

impl<T: Datastore> GraphManager<T> {
    fn setup_indexed_properties(&mut self) -> Result<()> {
        self.db.index_property(indradb::Identifier::new("slug")?)?;
        self.db.index_property(indradb::Identifier::new("db_id")?)?;
        Ok(())
    }

    fn get_vertex_by_slug(&self, slug: &str) -> Result<indradb::Vertex> {
        let query = indradb::VertexWithPropertyValueQuery::new(
            indradb::Identifier::new("slug")?,
            ijson!(slug),
        );
        match self.db.get(query)?.as_slice() {
            [QueryOutputValue::Vertices(vertices)] => match vertices.as_slice() {
                [vertex] => Ok(vertex.clone()),
                [] => Err(anyhow!("No vertex found with slug {}", slug)),
                _ => Err(anyhow!(
                    "Expected exactly one vertex with slug {}, found {}",
                    slug,
                    vertices.len()
                )),
            },
            _ => Err(anyhow!(
                "Unexpected output type when querying slug {}",
                slug
            )),
        }
    }

    fn get_vertex_by_name(&self, name: &str) -> Result<indradb::Vertex> {
        let slug = slugify(name);
        self.get_vertex_by_slug(&slug)
    }

    pub fn get_vertex_by_id(&self, id: uuid::Uuid) -> Result<indradb::Vertex> {
        let q = indradb::SpecificVertexQuery::single(id);
        match self.db.get(q)?.as_slice() {
            [QueryOutputValue::Vertices(vertices)] => {
                if let Some(v) = vertices.first() {
                    Ok(v.clone())
                } else {
                    Err(anyhow!("No vertex found with id {}", id))
                }
            }
            _ => Err(anyhow!(
                "Unexpected output type when fetching vertex {}",
                id
            )),
        }
    }

    pub fn get_vertex_db_id(&self, id: uuid::Uuid) -> Result<i64> {
        let q = indradb::SpecificVertexQuery::single(id)
            .properties()?
            .name(indradb::Identifier::new("db_id")?);

        match self.db.get(q)?.as_slice() {
            [QueryOutputValue::VertexProperties(props)] => {
                if let Some(vp) = props.first() {
                    if let Some(prop) = vp.props.first() {
                        if let Some(id) = prop.value.as_i64() {
                            return Ok(id);
                        }
                    }
                }
                Err(anyhow!("No db_id property found"))
            }
            _ => Err(anyhow!("Unexpected output type")),
        }
    }

    pub fn get_exercise_vert(&self, ex: &dbm::Exercise) -> Result<uuid::Uuid> {
        match self.get_exercise_by_db_id(ex.id) {
            Ok(exercise_vert) => Ok(exercise_vert.id),
            Err(_) => self.add_exercise(ex),
        }
    }

    fn get_edge_property_f64(
        &self,
        eq: indradb::SpecificEdgeQuery,
        prop_name: &str,
    ) -> Option<f64> {
        let pq = eq
            .properties()
            .ok()?
            .name(indradb::Identifier::new(prop_name).ok()?);
        if let Ok(result) = self.db.get(pq) {
            if let [QueryOutputValue::EdgeProperties(props)] = result.as_slice() {
                if let Some(ep) = props.first() {
                    if let Some(prop) = ep.props.first() {
                        return prop.value.as_f64();
                    }
                }
            }
        }
        None
    }

    fn get_edge_property_string(
        &self,
        eq: indradb::SpecificEdgeQuery,
        prop_name: &str,
    ) -> Option<String> {
        let pq = eq
            .properties()
            .ok()?
            .name(indradb::Identifier::new(prop_name).ok()?);
        if let Ok(result) = self.db.get(pq) {
            if let [QueryOutputValue::EdgeProperties(props)] = result.as_slice() {
                if let Some(ep) = props.first() {
                    if let Some(prop) = ep.props.first() {
                        return prop.value.as_str().map(|s| s.to_string());
                    }
                }
            }
        }
        None
    }

    fn get_edge_property_bool(
        &self,
        eq: indradb::SpecificEdgeQuery,
        prop_name: &str,
    ) -> Option<bool> {
        let pq = eq
            .properties()
            .ok()?
            .name(indradb::Identifier::new(prop_name).ok()?);
        if let Ok(result) = self.db.get(pq) {
            if let [QueryOutputValue::EdgeProperties(props)] = result.as_slice() {
                if let Some(ep) = props.first() {
                    if let Some(prop) = ep.props.first() {
                        return prop.value.as_bool();
                    }
                }
            }
        }
        None
    }

    fn read_involvement(&self, edge: &indradb::Edge) -> Result<MuscleInvolvement> {
        let eq = indradb::SpecificEdgeQuery::single(edge.clone());

        let scale_factor = self
            .get_edge_property_f64(eq.clone(), "scale_factor")
            .unwrap_or(0.5);

        let usage_type_str = self
            .get_edge_property_string(eq, "usage_type")
            .unwrap_or_else(|| "synergist".to_string());

        let usage_type = MuscleUsageType::from_str(&usage_type_str)?;

        Ok(MuscleInvolvement::new(scale_factor, usage_type))
    }

    pub fn add_muscle(&self, muscle: dbm::Muscle) -> Result<uuid::Uuid> {
        let v_id = self
            .db
            .create_vertex_from_type(indradb::Identifier::new("muscle")?)?;
        let q = indradb::SpecificVertexQuery::single(v_id);
        let slug = slugify(&muscle.name);
        self.db
            .set_properties(q.clone(), indradb::Identifier::new("slug")?, &ijson!(slug))?;
        self.db.set_properties(
            q.clone(),
            indradb::Identifier::new("db_id")?,
            &ijson!(muscle.id),
        )?;
        self.db.set_properties(
            q.clone(),
            indradb::Identifier::new("db_props")?,
            &ijson!(muscle),
        )?;
        Ok(v_id)
    }

    pub fn add_muscle_group(&self, group_name: &str) -> Result<uuid::Uuid> {
        let v_id = self
            .db
            .create_vertex_from_type(indradb::Identifier::new("muscle_group")?)?;
        let q = indradb::SpecificVertexQuery::single(v_id);
        let slug = slugify(group_name);
        self.db
            .set_properties(q.clone(), indradb::Identifier::new("slug")?, &ijson!(slug))?;
        Ok(v_id)
    }

    pub fn get_muscle_by_name(&self, name: &str) -> Result<indradb::Vertex> {
        self.get_vertex_by_name(name)
    }

    pub fn get_muscle_group_by_name(&self, name: &str) -> Result<indradb::Vertex> {
        self.get_vertex_by_name(name)
    }

    pub fn link_muscle_to_group(&self, group_id: uuid::Uuid, muscle_id: uuid::Uuid) -> Result<()> {
        let e = indradb::Edge::new(muscle_id, indradb::Identifier::new("member_of")?, group_id);
        self.db.create_edge(&e)?;
        Ok(())
    }

    pub fn get_muscles_in_group(&self, group_id: uuid::Uuid) -> Result<Vec<uuid::Uuid>> {
        let q = indradb::SpecificVertexQuery::single(group_id)
            .inbound()?
            .t(indradb::Identifier::new("member_of")?);

        let result = self.db.get(q)?;
        let edges = match result.as_slice() {
            [QueryOutputValue::Edges(edges)] => edges,
            _ => return Ok(Vec::new()),
        };

        Ok(edges.iter().map(|edge| edge.outbound_id).collect())
    }

    pub fn get_all_muscles_in_group(&self, source: uuid::Uuid) -> Result<Vec<uuid::Uuid>> {
        let mut muscles = Vec::new();
        let mut to_visit = vec![source];
        let mut visited = std::collections::HashSet::new();

        while let Some(current_id) = to_visit.pop() {
            if !visited.insert(current_id) {
                continue;
            }

            if self.is_muscle(current_id)? {
                muscles.push(current_id);
            } else {
                let children = self.get_muscles_in_group(current_id)?;
                to_visit.extend(children);
            }
        }

        Ok(muscles)
    }

    fn is_muscle(&self, vertex_id: uuid::Uuid) -> Result<bool> {
        let q = indradb::SpecificVertexQuery::single(vertex_id);
        match self.db.get(q)?.as_slice() {
            [QueryOutputValue::Vertices(verts)] => {
                Ok(verts.first().unwrap().t == indradb::Identifier::new("muscle")?)
            }
            _ => Ok(false),
        }
    }

    pub fn get_muscle_db_ids_in_group(&self, source: uuid::Uuid) -> Result<Vec<i32>> {
        let muscle_vertex_ids = self.get_all_muscles_in_group(source)?;
        let mut db_ids = Vec::new();

        for vertex_id in muscle_vertex_ids {
            let q = indradb::SpecificVertexQuery::single(vertex_id)
                .properties()?
                .name(indradb::Identifier::new("db_id")?);

            if let [QueryOutputValue::VertexProperties(vert_props)] = self.db.get(q)?.as_slice() {
                if let Some(vp) = vert_props.first() {
                    if let Some(prop) = vp.props.first() {
                        if let Some(id) = prop.value.as_i64() {
                            db_ids.push(id as i32);
                        }
                    }
                }
            }
        }

        Ok(db_ids)
    }

    pub fn add_equipment(
        &self,
        name: &str,
        description: Option<&str>,
        db_id: i64,
    ) -> Result<uuid::Uuid> {
        let v_id = self
            .db
            .create_vertex_from_type(indradb::Identifier::new("equipment")?)?;
        let q = indradb::SpecificVertexQuery::single(v_id);
        let slug = slugify(name);

        self.db
            .set_properties(q.clone(), indradb::Identifier::new("slug")?, &ijson!(slug))?;
        self.db
            .set_properties(q.clone(), indradb::Identifier::new("name")?, &ijson!(name))?;
        self.db.set_properties(
            q.clone(),
            indradb::Identifier::new("db_id")?,
            &ijson!(db_id),
        )?;

        if let Some(desc) = description {
            self.db.set_properties(
                q.clone(),
                indradb::Identifier::new("description")?,
                &ijson!(desc),
            )?;
        }

        Ok(v_id)
    }

    pub fn get_equipment_by_name(&self, name: &str) -> Result<indradb::Vertex> {
        self.get_vertex_by_name(name)
    }

    pub fn add_exercise(&self, exercise: &dbm::Exercise) -> Result<uuid::Uuid> {
        let v_id = self
            .db
            .create_vertex_from_type(indradb::Identifier::new("exercise")?)?;
        let q = indradb::SpecificVertexQuery::single(v_id);

        self.db.set_properties(
            q.clone(),
            indradb::Identifier::new("slug")?,
            &ijson!(exercise.slug),
        )?;
        self.db.set_properties(
            q.clone(),
            indradb::Identifier::new("name")?,
            &ijson!(exercise.name),
        )?;
        self.db.set_properties(
            q.clone(),
            indradb::Identifier::new("db_id")?,
            &ijson!(exercise.id),
        )?;

        if let Some(ref desc) = exercise.description {
            self.db.set_properties(
                q.clone(),
                indradb::Identifier::new("description")?,
                &ijson!(desc),
            )?;
        }

        Ok(v_id)
    }

    pub fn get_exercise_by_name(&self, name: &str) -> Result<indradb::Vertex> {
        self.get_vertex_by_name(name)
    }

    pub fn get_exercise_by_db_id(&self, db_id: i64) -> Result<indradb::Vertex> {
        let query = indradb::VertexWithPropertyValueQuery::new(
            indradb::Identifier::new("db_id")?,
            ijson!(db_id),
        );

        match self.db.get(query)?.as_slice() {
            [QueryOutputValue::Vertices(vertices)] => {
                for vertex in vertices {
                    if vertex.t == indradb::Identifier::new("exercise")? {
                        return Ok(vertex.clone());
                    }
                }
                Err(anyhow!("No exercise found with db_id {}", db_id))
            }
            _ => Err(anyhow!("No exercise found with db_id {}", db_id)),
        }
    }

    pub fn link_exercise_to_equipment(
        &self,
        exercise_id: uuid::Uuid,
        equipment_id: uuid::Uuid,
        is_required: bool,
    ) -> Result<()> {
        let edge = indradb::Edge::new(
            exercise_id,
            indradb::Identifier::new("uses_equipment")?,
            equipment_id,
        );
        self.db.create_edge(&edge)?;

        let eq = indradb::SpecificEdgeQuery::single(edge);
        self.db.set_properties(
            eq,
            indradb::Identifier::new("is_required")?,
            &ijson!(is_required),
        )?;

        let reverse_edge = indradb::Edge::new(
            equipment_id,
            indradb::Identifier::new("used_by_exercise")?,
            exercise_id,
        );
        self.db.create_edge(&reverse_edge)?;

        Ok(())
    }

    pub fn get_equipment_for_exercise(&self, exercise_id: uuid::Uuid) -> Result<Vec<uuid::Uuid>> {
        let q = indradb::SpecificVertexQuery::single(exercise_id)
            .outbound()?
            .t(indradb::Identifier::new("uses_equipment")?);

        match self.db.get(q)?.as_slice() {
            [QueryOutputValue::Edges(edges)] => Ok(edges.iter().map(|e| e.inbound_id).collect()),
            _ => Ok(vec![]),
        }
    }

    pub fn get_required_equipment_db_ids_for_exercise(
        &self,
        exercise_id: uuid::Uuid,
    ) -> Result<Vec<i64>> {
        let q = indradb::SpecificVertexQuery::single(exercise_id)
            .outbound()?
            .t(indradb::Identifier::new("uses_equipment")?);

        let result = self.db.get(q)?;
        let edges = match result.as_slice() {
            [QueryOutputValue::Edges(edges)] => edges,
            _ => return Ok(vec![]),
        };

        let mut required_db_ids = Vec::new();
        for edge in edges {
            let eq = indradb::SpecificEdgeQuery::single(edge.clone());
            let is_required = self
                .get_edge_property_bool(eq, "is_required")
                .unwrap_or(false);

            if is_required {
                // Get equipment vertex db_id
                let equipment_vertex = self.get_vertex_by_id(edge.inbound_id)?;
                let eq_q = indradb::SpecificVertexQuery::single(equipment_vertex.id)
                    .properties()?
                    .name(indradb::Identifier::new("db_id")?);

                if let [QueryOutputValue::VertexProperties(vert_props)] =
                    self.db.get(eq_q)?.as_slice()
                {
                    if let Some(vp) = vert_props.first() {
                        if let Some(prop) = vp.props.first() {
                            if let Some(db_id) = prop.value.as_i64() {
                                required_db_ids.push(db_id);
                            }
                        }
                    }
                }
            }
        }

        Ok(required_db_ids)
    }

    pub fn get_exercises_for_equipment(&self, equipment_id: uuid::Uuid) -> Result<Vec<uuid::Uuid>> {
        let q = indradb::SpecificVertexQuery::single(equipment_id)
            .outbound()?
            .t(indradb::Identifier::new("used_by_exercise")?);

        match self.db.get(q)?.as_slice() {
            [QueryOutputValue::Edges(edges)] => Ok(edges.iter().map(|e| e.inbound_id).collect()),
            _ => Ok(vec![]),
        }
    }

    pub fn link_exercise_to_muscle(
        &self,
        exercise_id: uuid::Uuid,
        muscle_id: uuid::Uuid,
        involvement: MuscleInvolvement,
    ) -> Result<()> {
        let edge = indradb::Edge::new(
            exercise_id,
            indradb::Identifier::new("targets_muscle")?,
            muscle_id,
        );
        self.db.create_edge(&edge)?;

        let eq = indradb::SpecificEdgeQuery::single(edge.clone());
        self.db.set_properties(
            eq.clone(),
            indradb::Identifier::new("scale_factor")?,
            &ijson!(involvement.scale_factor),
        )?;
        self.db.set_properties(
            eq,
            indradb::Identifier::new("usage_type")?,
            &ijson!(involvement.usage_type.as_str()),
        )?;

        let reverse_edge = indradb::Edge::new(
            muscle_id,
            indradb::Identifier::new("worked_by")?,
            exercise_id,
        );
        self.db.create_edge(&reverse_edge)?;

        let req = indradb::SpecificEdgeQuery::single(reverse_edge);
        self.db.set_properties(
            req.clone(),
            indradb::Identifier::new("scale_factor")?,
            &ijson!(involvement.scale_factor),
        )?;
        self.db.set_properties(
            req,
            indradb::Identifier::new("usage_type")?,
            &ijson!(involvement.usage_type.as_str()),
        )?;

        Ok(())
    }

    pub fn get_muscles_for_exercise(
        &self,
        exercise_id: uuid::Uuid,
    ) -> Result<Vec<(uuid::Uuid, MuscleInvolvement)>> {
        let q = indradb::SpecificVertexQuery::single(exercise_id)
            .outbound()?
            .t(indradb::Identifier::new("targets_muscle")?);

        let results = self.db.get(q)?;
        let edges = match results.as_slice() {
            [QueryOutputValue::Edges(edges)] => edges,
            _ => return Ok(vec![]),
        };

        edges
            .iter()
            .map(|edge| {
                let involvement = self.read_involvement(edge)?;
                Ok((edge.inbound_id, involvement))
            })
            .collect()
    }

    /// Gets muscles for an exercise, returning SQL db_ids instead of graph UUIDs.
    /// This is useful for matching against target muscle distributions in workout planning.
    pub fn get_muscles_with_db_ids_for_exercise(
        &self,
        exercise_id: uuid::Uuid,
    ) -> Result<Vec<(i64, MuscleInvolvement)>> {
        let q = indradb::SpecificVertexQuery::single(exercise_id)
            .outbound()?
            .t(indradb::Identifier::new("targets_muscle")?);

        let results = self.db.get(q)?;
        let edges = match results.as_slice() {
            [QueryOutputValue::Edges(edges)] => edges,
            _ => return Ok(vec![]),
        };

        let mut result = Vec::new();
        for edge in edges {
            let involvement = self.read_involvement(edge)?;
            // get the db_id property from the muscle vertex
            let muscle_vertex = self.get_vertex_by_id(edge.inbound_id)?;
            let props_q = indradb::SpecificVertexQuery::single(muscle_vertex.id)
                .properties()?
                .name(indradb::Identifier::new("db_id")?);

            if let Ok(prop_values) = self.db.get(props_q) {
                if let [QueryOutputValue::VertexProperties(vert_props)] = prop_values.as_slice() {
                    if let Some(vp) = vert_props.first() {
                        if let Some(prop) = vp.props.first() {
                            if let Some(db_id) = prop.value.as_i64() {
                                result.push((db_id, involvement));
                            }
                        }
                    }
                }
            }
        }
        Ok(result)
    }

    pub fn get_exercises_for_muscle(
        &self,
        muscle_id: uuid::Uuid,
    ) -> Result<Vec<(uuid::Uuid, MuscleInvolvement)>> {
        let q = indradb::SpecificVertexQuery::single(muscle_id)
            .outbound()?
            .t(indradb::Identifier::new("worked_by")?);

        let results = self.db.get(q)?;
        let edges = match results.as_slice() {
            [QueryOutputValue::Edges(edges)] => edges,
            _ => return Ok(vec![]),
        };

        edges
            .iter()
            .map(|edge| {
                let involvement = self.read_involvement(edge)?;
                Ok((edge.inbound_id, involvement))
            })
            .collect()
    }

    /// Adds a movement pattern vertex to the graph.
    /// Returns the UUID of the created vertex.
    pub fn add_movement_pattern(&self, pattern: ExercisePatternType) -> Result<uuid::Uuid> {
        let v_id = self
            .db
            .create_vertex_from_type(indradb::Identifier::new("movement_pattern")?)?;
        let q = indradb::SpecificVertexQuery::single(v_id);
        let slug = pattern.as_str();

        self.db
            .set_properties(q.clone(), indradb::Identifier::new("slug")?, &ijson!(slug))?;
        self.db.set_properties(
            q.clone(),
            indradb::Identifier::new("pattern_type")?,
            &ijson!(slug),
        )?;

        Ok(v_id)
    }

    /// Gets a movement pattern vertex by its type.
    /// Returns the vertex if found.
    pub fn get_movement_pattern(&self, pattern: ExercisePatternType) -> Result<indradb::Vertex> {
        let slug = pattern.as_str();
        self.get_vertex_by_slug(slug)
    }

    /// Gets or creates a movement pattern vertex.
    /// Returns the UUID of the vertex.
    pub fn get_or_create_movement_pattern(
        &self,
        pattern: ExercisePatternType,
    ) -> Result<uuid::Uuid> {
        match self.get_movement_pattern(pattern) {
            Ok(vertex) => Ok(vertex.id),
            Err(_) => self.add_movement_pattern(pattern),
        }
    }

    /// Links an exercise to a movement pattern.
    /// Creates the movement pattern vertex if it doesn't exist.
    pub fn link_exercise_to_movement_pattern(
        &self,
        exercise_id: uuid::Uuid,
        pattern: ExercisePatternType,
    ) -> Result<()> {
        let pattern_id = self.get_or_create_movement_pattern(pattern)?;

        let edge = indradb::Edge::new(
            exercise_id,
            indradb::Identifier::new("has_movement_pattern")?,
            pattern_id,
        );
        self.db.create_edge(&edge)?;

        let reverse_edge = indradb::Edge::new(
            pattern_id,
            indradb::Identifier::new("pattern_of_exercise")?,
            exercise_id,
        );
        self.db.create_edge(&reverse_edge)?;

        Ok(())
    }

    /// Gets the movement pattern for an exercise.
    /// Returns Unknown if no pattern is found.
    pub fn get_movement_pattern_for_exercise(
        &self,
        exercise_id: uuid::Uuid,
    ) -> Result<ExercisePatternType> {
        // Try to build the query, return Unknown on any error
        let q = match (|| -> Result<_, anyhow::Error> {
            Ok(indradb::SpecificVertexQuery::single(exercise_id)
                .outbound()?
                .t(indradb::Identifier::new("has_movement_pattern")?))
        })() {
            Ok(q) => q,
            Err(_) => return Ok(ExercisePatternType::Unknown),
        };

        let result = match self.db.get(q) {
            Ok(r) => r,
            Err(_) => return Ok(ExercisePatternType::Unknown),
        };

        let edges = match result.as_slice() {
            [QueryOutputValue::Edges(edges)] => edges,
            _ => return Ok(ExercisePatternType::Unknown),
        };

        let pattern_vertex_id = match edges.first() {
            Some(edge) => edge.inbound_id,
            None => return Ok(ExercisePatternType::Unknown),
        };

        // Get the pattern_type property from the movement pattern vertex
        let prop_query = match (|| -> Result<_, anyhow::Error> {
            Ok(indradb::SpecificVertexQuery::single(pattern_vertex_id)
                .properties()?
                .name(indradb::Identifier::new("pattern_type")?))
        })() {
            Ok(q) => q,
            Err(_) => return Ok(ExercisePatternType::Unknown),
        };

        let prop_result = match self.db.get(prop_query) {
            Ok(r) => r,
            Err(_) => return Ok(ExercisePatternType::Unknown),
        };

        match prop_result.as_slice() {
            [QueryOutputValue::VertexProperties(vert_props)] => {
                if let Some(vp) = vert_props.first() {
                    if let Some(prop) = vp.props.first() {
                        if let Some(pattern_str) = prop.value.as_str() {
                            return Ok(ExercisePatternType::from_str(pattern_str));
                        }
                    }
                }
                Ok(ExercisePatternType::Unknown)
            }
            _ => Ok(ExercisePatternType::Unknown),
        }
    }

    /// Gets all exercises that have a specific movement pattern.
    pub fn get_exercises_for_movement_pattern(
        &self,
        pattern: ExercisePatternType,
    ) -> Result<Vec<uuid::Uuid>> {
        let pattern_vertex = self.get_movement_pattern(pattern)?;

        let q = indradb::SpecificVertexQuery::single(pattern_vertex.id)
            .outbound()?
            .t(indradb::Identifier::new("pattern_of_exercise")?);

        match self.db.get(q)?.as_slice() {
            [QueryOutputValue::Edges(edges)] => Ok(edges.iter().map(|e| e.inbound_id).collect()),
            _ => Ok(vec![]),
        }
    }
}
