{
  description = "Dev shell with ephemeral Postgres + Diesel + Neo4j";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
  };

  outputs =
    { self, nixpkgs }:
    let
      # List of supported systems
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      # Helper to map over all systems
      forAllSystems =
        f:
        nixpkgs.lib.genAttrs supportedSystems (
          system:
          let
            pkgs = import nixpkgs { inherit system; };
          in
          f system pkgs
        );
    in
    {
      packages = forAllSystems (
        system: pkgs: {
          yoku-cli = pkgs.rustPlatform.buildRustPackage {
            pname = "yoku-cli";
            version = "0.1.0";

            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            nativeBuildInputs = with pkgs; [ pkg-config ];

            buildInputs = with pkgs; [
              openssl
              sqlite
            ];

            cargoBuildFlags = [
              "--lib"
            ];

            meta = with pkgs.lib; {
              description = "Yoku workout tracker CLI";
              license = licenses.mit;
              platforms = supportedSystems;
            };
          };

          default = self.packages.${system}.yoku-cli;
        }
      );

      devShells = forAllSystems (
        system: pkgs: {
          default = pkgs.mkShell {
            buildInputs = with pkgs; [
              diesel-cli
              rustc
              cargo
              neo4j
              sqlite
            ];

            shellHook = ''
              echo "Starting Neo4j (ephemeral)..."
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

              diesel migration run || true

              cleanup() {
                kill $NEO4J_PID >/dev/null 2>&1 || true
                rm -rf "$NEO4J_TMP"
              }
              trap cleanup EXIT
            '';
          };
        }
      );
    };
}
