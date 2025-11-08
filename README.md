# Yoku - Intelligent Workout Tracker

A natural language workout tracking application that uses graph databases to understand exercise relationships and provide intelligent weight/rep suggestions.

## Overview

Yoku takes natural language input (e.g., "3x5 bench press at 225 lbs") and stores it in a structured database. It uses a graph database to model relationships between exercises, enabling intelligent suggestions for weights and reps even for exercises the user hasn't performed before.

### Example Use Case

If a user has done **barbell bench press** before, the graph links it to **incline barbell dumbbell press** with relationship metadata. When the user wants to try the incline variation, Yoku can suggest appropriate weights for a given rep range and RPE based on:
- Historical performance on related exercises
- Exercise relationship strength and similarity
- User's training patterns and progression

## Features

### Core Features

- [ ] **Natural Language Input Parsing**
  - Parse workout entries from free-form text
  - Extract: exercise name, sets, reps, weight, RPE, rest time, notes
  - Support multiple formats and variations

- [ ] **Structured Data Storage**
  - Workout sessions with timestamps
  - Exercises with metadata (muscle groups, equipment, movement patterns)
  - Sets with weight, reps, RPE, rest duration
  - User profile and preferences

- [ ] **Graph Database for Exercise Relationships**
  - Model exercises as nodes with properties
  - Create edges representing relationships (similarity, progression, variation)
  - Edge weights/strengths for relationship quantification
  - Support for multiple relationship types (muscle group, movement pattern, equipment, difficulty)

- [ ] **Intelligent Weight/Rep Suggestions**
  - Suggest weights for new exercises based on related exercise history
  - Account for rep ranges, RPE targets, and training goals
  - Consider progression patterns and user strength levels
  - Provide confidence scores for suggestions

- [ ] **Exercise Discovery and Linking**
  - Auto-link exercises mentioned in natural language to graph nodes
  - Handle exercise name variations and aliases
  - Suggest similar exercises when user types something new

- [ ] **Historical Analysis**
  - Track progression over time
  - Identify strength trends
  - Calculate estimated 1RM and volume metrics

### Advanced Features (Future)

- [ ] **LLM-Powered Exercise Organization**
  - Use LLM to automatically categorize and link exercises
  - Generate exercise descriptions and form cues
  - Suggest workout programs based on goals

- [ ] **Pre-populated Exercise Database**
  - Seed graph with common exercises and relationships
  - Include standard progressions and variations
  - Community-contributed exercise data

- [ ] **Multi-user Support**
  - User authentication and data isolation
  - Share workout templates and programs

- [ ] **Export/Import**
  - Export data in various formats (CSV, JSON)
  - Import from other fitness apps
  - Backup and restore functionality

- [ ] **Visualization**
  - Graph visualization of exercise relationships
  - Progress charts and graphs
  - Workout history timeline

## Implementation Roadmap

### Phase 1: Foundation (Weeks 1-2)
**Goal**: Basic CLI with structured data storage

1. **Database Schema Design**
   - Design SQL schema for workouts, exercises, sets
   - Set up database migrations
   - Implement basic CRUD operations

2. **Natural Language Parser (Basic)**
   - Implement regex-based parser for common patterns
   - Support basic formats: "3x5 bench 225", "bench press 3 sets 5 reps 225 lbs"
   - Extract core fields: exercise, sets, reps, weight

3. **CLI Interface**
   - Commands: `add`, `list`, `show`, `delete`
   - Basic error handling and validation

**Deliverable**: Can add and view workouts via CLI

### Phase 2: Graph Database Integration (Weeks 3-4)
**Goal**: Exercise relationship modeling

1. **Graph Database Setup**
   - Choose and set up graph database (Neo4j recommended)
   - Design node and relationship schemas
   - Create initial exercise nodes

2. **Exercise Linking**
   - Manual exercise creation with metadata
   - Link exercises to graph nodes
   - Handle exercise name normalization

3. **Relationship Management**
   - Create relationship types (similar, variation, progression)
   - Implement relationship strength/weight system
   - Basic graph queries for related exercises

**Deliverable**: Exercises stored in both SQL and graph, with relationships

### Phase 3: Intelligent Suggestions (Weeks 5-6)
**Goal**: Weight/rep prediction algorithm

1. **Suggestion Algorithm**
   - Implement weight prediction based on related exercises
   - Calculate relationship-weighted averages
   - Account for rep ranges and RPE

2. **Historical Analysis**
   - Track user progression patterns
   - Calculate strength metrics (estimated 1RM, volume)
   - Identify trends and plateaus

3. **Suggestion API**
   - CLI command: `suggest <exercise> <reps> <rpe>`
   - Display confidence scores and reasoning

**Deliverable**: Can suggest weights for new exercises

### Phase 4: Enhanced NLP (Weeks 7-8)
**Goal**: Robust natural language understanding

1. **LLM Integration**
   - Integrate LLM API (OpenAI, Anthropic, or local)
   - Use LLM as fallback when regex fails
   - Improve exercise name disambiguation

2. **Parser Improvements**
   - Handle more complex input formats
   - Extract additional fields (RPE, rest time, notes)
   - Better error messages and suggestions

3. **Exercise Auto-linking**
   - Use LLM to match user input to existing exercises
   - Suggest exercises when input doesn't match
   - Handle typos and variations

**Deliverable**: Robust NLP with LLM fallback

### Phase 5: Exercise Database & Organization (Weeks 9-10)
**Goal**: Rich exercise knowledge base

1. **Pre-populated Database**
   - Seed graph with 100+ common exercises
   - Create relationships between exercises
   - Include metadata (muscle groups, equipment, difficulty)

2. **LLM-Powered Organization**
   - Use LLM to automatically categorize exercises
   - Generate relationships between exercises
   - Create exercise descriptions and cues

3. **Exercise Management**
   - CLI commands for exercise CRUD
   - View exercise relationships
   - Suggest similar exercises

**Deliverable**: Rich exercise database with intelligent organization

### Phase 6: Polish & Advanced Features (Weeks 11-12)
**Goal**: Production-ready application

1. **Error Handling & Validation**
   - Comprehensive error handling
   - Input validation and sanitization
   - User-friendly error messages

2. **Performance Optimization**
   - Query optimization
   - Caching for frequent operations
   - Batch operations for bulk imports

3. **Documentation & Testing**
   - Comprehensive test coverage
   - User documentation
   - API documentation

4. **Advanced Features**
   - Export/import functionality
   - Progress visualization
   - Workout templates

**Deliverable**: Production-ready application

## Technology Stack Recommendations

### Natural Language Processing

#### Option 1: Regex + Nom Parser Combinators
**Packages**: `regex`, `nom`

**Pros**:
- Fast and lightweight
- No external dependencies
- Predictable behavior
- Good for structured patterns

**Cons**:
- Limited flexibility for varied input
- Requires manual pattern definition
- Doesn't handle typos or variations well

**Use Case**: Primary parser for common, well-structured input patterns

#### Option 2: LLM API Integration
**Packages**: `openai`, `anthropic`, `llm-chain`, `async-openai`

**Pros**:
- Handles varied and ambiguous input
- Can understand context and intent
- Good for exercise name disambiguation
- Flexible for future enhancements

**Cons**:
- Requires API key and internet connection
- Latency and cost per request
- Less predictable than regex
- Privacy concerns with user data

**Use Case**: Fallback parser and exercise matching

#### Option 3: Hybrid Approach (Recommended)
**Packages**: `regex`, `nom`, `openai` or `anthropic`

**Pros**:
- Fast for common cases (regex)
- Flexible for edge cases (LLM)
- Cost-effective (minimize LLM calls)
- Best of both worlds

**Cons**:
- More complex implementation
- Need to manage two parsing paths

**Use Case**: Primary regex parser with LLM fallback

### Structured Database (Workouts, Sets, Exercises)

#### Option 1: PostgreSQL with SQLx
**Packages**: `sqlx`, `sqlx-cli` (for migrations)

**Pros**:
- Mature and battle-tested
- Excellent Rust support with `sqlx`
- Type-safe queries with compile-time checking
- ACID compliance
- Good performance for structured data
- Rich ecosystem and tooling

**Cons**:
- Requires separate database server
- More setup complexity
- Overkill for simple use cases

**Use Case**: Recommended for production, multi-user, or complex queries

#### Option 2: SQLite with Rusqlite
**Packages**: `rusqlite`

**Pros**:
- Zero configuration (file-based)
- Perfect for single-user applications
- No separate server needed
- Good performance for moderate data
- Easy backup (just copy file)

**Cons**:
- Limited concurrency
- Not ideal for high write loads
- Less feature-rich than PostgreSQL

**Use Case**: Recommended for CLI tool, single-user, or development

#### Option 3: Diesel ORM
**Packages**: `diesel`, `diesel_migrations`

**Pros**:
- Type-safe ORM
- Compile-time query checking
- Good migration system
- Supports PostgreSQL, MySQL, SQLite

**Cons**:
- Steeper learning curve
- More boilerplate
- Compile times can be slower
- Less flexible than SQLx for complex queries

**Use Case**: Good if you prefer ORM-style abstractions

**Recommendation**: Start with **SQLite + Rusqlite** for simplicity, migrate to **PostgreSQL + SQLx** if needed for production.

### Graph Database

#### Option 1: Neo4j with Neo4rs
**Packages**: `neo4rs`

**Pros**:
- Industry-standard graph database
- Excellent Rust support with `neo4rs`
- Rich query language (Cypher)
- Great for complex relationship queries
- Mature ecosystem
- Can run embedded or as service

**Cons**:
- Requires separate database server (or embedded setup)
- Additional infrastructure
- Learning curve for Cypher queries

**Use Case**: Recommended for production with complex relationship queries

#### Option 2: In-Memory Graph with Petgraph
**Packages**: `petgraph`

**Pros**:
- Pure Rust, no external dependencies
- Fast for in-memory operations
- Good for smaller graphs
- Simple API

**Cons**:
- No persistence (data lost on restart)
- Limited query capabilities
- Not suitable for large graphs
- Need to implement persistence layer

**Use Case**: Good for prototyping or small datasets with custom persistence

#### Option 3: ArangoDB
**Packages**: `arangors`

**Pros**:
- Multi-model database (document + graph)
- Can store both structured and graph data
- Good Rust support
- Flexible data model

**Cons**:
- Less mature than Neo4j
- Smaller community
- More complex setup

**Use Case**: Good if you want unified database for both structured and graph data

**Recommendation**: **Neo4j with Neo4rs** for production, **Petgraph** for prototyping.

### Serialization & Data Handling

#### Serde Ecosystem
**Packages**: `serde`, `serde_json`, `serde_yaml`, `serde_derive`

**Pros**:
- Standard Rust serialization
- Excellent performance
- Supports many formats
- Widely used and well-maintained

**Use Case**: Essential for all data serialization needs

### CLI Framework

#### Option 1: Clap
**Packages**: `clap` (with `derive` feature)

**Pros**:
- Most popular Rust CLI framework
- Excellent documentation
- Rich features (auto-completion, help generation)
- Type-safe argument parsing
- Great developer experience

**Cons**:
- Can be verbose for simple CLIs
- Larger binary size

**Use Case**: Recommended for all CLI needs

#### Option 2: StructOpt (deprecated, use Clap derive)
**Packages**: N/A (deprecated)

**Note**: Use Clap's derive feature instead

### Date/Time Handling

#### Chrono
**Packages**: `chrono`

**Pros**:
- Standard Rust date/time library
- Rich functionality
- Timezone support
- Well-maintained

**Use Case**: Essential for workout timestamps and date calculations

### Error Handling

#### Anyhow + Thiserror
**Packages**: `anyhow`, `thiserror`

**Pros**:
- `anyhow` for application errors (flexible)
- `thiserror` for library errors (structured)
- Great error context and chaining
- Standard Rust error handling patterns

**Use Case**: Recommended error handling approach

### Configuration

#### Config
**Packages**: `config`

**Pros**:
- Supports multiple formats (TOML, JSON, YAML, etc.)
- Environment variable support
- Hierarchical configuration
- Type-safe

**Use Case**: Application configuration management

### Logging

#### Tracing
**Packages**: `tracing`, `tracing-subscriber`

**Pros**:
- Modern structured logging
- Great for async code
- Rich ecosystem
- Better than `log` for complex applications

**Use Case**: Recommended for application logging

## Recommended Initial Stack

For MVP/Development:
- **NLP**: `regex` + `nom` (primary), `openai` or `anthropic` (fallback)
- **Structured DB**: `rusqlite` (SQLite)
- **Graph DB**: `neo4rs` (Neo4j) or `petgraph` (for prototyping)
- **CLI**: `clap` with derive
- **Serialization**: `serde` + `serde_json`
- **Date/Time**: `chrono`
- **Errors**: `anyhow` + `thiserror`
- **Logging**: `tracing` + `tracing-subscriber`
- **Config**: `config`

For Production:
- **Structured DB**: Migrate to `sqlx` + PostgreSQL if needed
- **Graph DB**: `neo4rs` + Neo4j server
- Add caching layer if needed
- Consider `tokio` for async operations if building API

## Project Structure

```
yoku/
├── yoku-core/          # Core library with business logic
│   ├── src/
│   │   ├── db/         # Database operations
│   │   ├── graph/      # Graph database operations
│   │   ├── parser/     # NLP parsing
│   │   ├── suggest/    # Weight/rep suggestion algorithm
│   │   └── models/     # Data models
│   └── Cargo.toml
├── yoku-cli/           # CLI application
│   ├── src/
│   │   └── main.rs
│   └── Cargo.toml
└── Cargo.toml          # Workspace manifest
```

## Getting Started

1. Set up database (SQLite for dev, PostgreSQL for prod)
2. Set up graph database (Neo4j)
3. Install dependencies: `cargo build`
4. Run migrations
5. Start using: `cargo run --bin yoku-cli add "3x5 bench press 225"`

## License

[Your License Here]
