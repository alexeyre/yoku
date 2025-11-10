{
  description = "Dev shell with ephemeral Postgres + Diesel";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-25.05-darwin";
  };

  outputs = { self, nixpkgs }: let
    system = "aarch64-darwin";
    pkgs = import nixpkgs { inherit system; };
  in {
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
        cargoBuildFlags = [ "--bin" "yoku-cli" "-p" "yoku-cli" ];

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
      ];

      # Postgres environment
      shellHook = ''
        export PGDATA=$(mktemp -d)
        export DATABASE_URL=postgres://postgres:postgres@localhost:5432/dev_db

        echo "Starting temporary PostgreSQL in $PGDATA..."
        initdb --username=postgres > /dev/null
        pg_ctl -o "-F -p 5432" -D $PGDATA start > /dev/null
        createdb -U postgres dev_db
        echo "PostgreSQL running — DATABASE_URL=$DATABASE_URL"

        diesel migration run

        cleanup() {
          echo "Stopping Postgres..."
          pg_ctl -D $PGDATA stop > /dev/null
          rm -rf "$PGDATA"
        }
        trap cleanup EXIT
      '';
    };
  };
}
