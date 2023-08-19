set shell := ["powershell.exe", "-c"]

export DATABASE_URL := "sqlite:schema.db"
export BACKEND_URL := "/api"
export RUST_BACKTRACE := "1"

run-back:
	cargo run --bin nndb-backend -- config-dev.toml

serve-front *args:
	trunk serve -d target/dist --proxy-rewrite={{BACKEND_URL}} --proxy-backend=http://127.0.0.1:8081 {{args}} frontend/index.html

hx:
	hx

[no-cd]
run *args:
	{{args}}