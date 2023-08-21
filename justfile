set shell := ["powershell.exe", "-c"]

export DATABASE_URL := "sqlite:schema.db"
export RUST_BACKTRACE := "1"
export BACKEND_URL := "/api"

# Run backend in debug mode
run-back:
	cargo run --bin nndb-backend -- config-dev.toml

# Serve frontend with Trunk
serve-front *args:
	trunk serve -d target/dist --proxy-rewrite={{BACKEND_URL}} --proxy-backend=http://127.0.0.1:8081 {{args}} frontend/index.html

# Launch helix with environment vars
hx:
	hx

# Run arbitrary command with environment vars
[no-cd]
run *args:
	{{args}}
	
# Create development database
create-dev-db:
	sqlx database create --database-url {{DATABASE_URL}}
	sqlx migrate run --database-url {{DATABASE_URL}} --source backend/migrations
	
# Override url to root and build frontend in release mode
build-front $BACKEND_URL="":
	trunk build -d target/release/static --release frontend/index.html

# Build backend in release mode
build-back:
	cargo build --bin nndb-backend --release

# Build frontend, backend, and pack artifacts
[windows]
pack out="./dist": build-front build-back
	if (test-path {{out}}) { rm {{out}} -Recurse }
	mkdir {{out}}
	mkdir {{out}}/pool
	mkdir {{out}}/thumb
	mkdir {{out}}/input
	cp target/release/nndb-backend.exe {{out}}/nndb.exe
	cp config.toml {{out}}/config.toml
	cp target/release/static {{out}}/static -Recurse
