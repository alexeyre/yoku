{
  description = "Dev shell with ephemeral Postgres + Diesel";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-25.05-darwin";
  };

  outputs =
    { self, nixpkgs }:
    let
      system = "aarch64-darwin";
      pkgs = import nixpkgs { inherit system; };
    in
    {
      packages.${system} = {
        yoku-cli = pkgs.rustPlatform.buildRustPackage {
          pname = "yoku-cli";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            openssl
            postgresql
          ];

          # Only build the yoku-cli binary
          cargoBuildFlags = [
            "--bin"
            "yoku-cli"
            "-p"
            "yoku-cli"
          ];

          meta = with pkgs.lib; {
            description = "Yoku workout tracker CLI";
            license = licenses.mit;
          };
        };

        default = self.packages.${system}.yoku-cli;
      };

      devShells.${system}.default = pkgs.mkShell {

        nativeBuildInputs = with pkgs; [
          postgresql
        ];

        buildInputs = with pkgs; [
          diesel-cli
          rustc
          cargo
          neo4j
        ];

        # Postgres + Neo4j environment
        shellHook = ''
          export PGDATA=$(mktemp -d)
          export DATABASE_URL=postgres://postgres:postgres@localhost:5432/dev_db

          echo "Starting temporary PostgreSQL in $PGDATA..."
          initdb --username=postgres > /dev/null
          pg_ctl -o "-F -p 5432" -D $PGDATA start > /dev/null
          createdb -U postgres dev_db
          echo "PostgreSQL running — DATABASE_URL=$DATABASE_URL"

          # If neo4j is available in the devShell, attempt to start a local ephemeral instance.
          # This tries a safe sequence: prefer `neo4j start`, fall back to `neo4j console &`.
          # If neo4j isn't present, we print instructions instead of failing the shell.
          echo "Starting Neo4j (ephemeral) for development..."
          # Note: Neo4j manages its own data directory; we don't attempt to mutate system-wide config here.
          # We set environment variables the application expects for connection.
          export NEO4J_TMP=$(mktemp -d)
          export NEO4J_HOME="$NEO4J_TMP"
          export NEO4J_CONF="$NEO4J_TMP/conf"
          mkdir -p "$NEO4J_CONF"
          cp -r --no-preserve=mode,ownership ${pkgs.neo4j}/share/neo4j/conf/* "$NEO4J_CONF"
          export NEO4J_USER=neo4j
          export NEO4J_PASSWORD=devpass1
          export NEO4J_HOST="bolt://localhost:7687"
          echo "dbms.default_listen_address=0.0.0.0" >> "$NEO4J_CONF/neo4j.conf"
          echo "dbms.connector.bolt.listen_address=:7687" >> "$NEO4J_CONF/neo4j.conf"
          echo "dbms.security.auth_enabled=false" >> "$NEO4J_CONF/neo4j.conf"
          neo4j start >/dev/null &
          export NEO4J_PID=$!
          # Run diesel migrations after databases are up
          diesel migration run

          cleanup() {
            echo "Stopping Postgres..."
            pg_ctl -D $PGDATA stop > /dev/null || true
            rm -rf "$PGDATA"
            kill $NEO4J_PID >/dev/null 2>&1 || true
            rm -rf "$NEO4J_TMP"
          }
          trap cleanup EXIT
        '';
      };
    };
}
