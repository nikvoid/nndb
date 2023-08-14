set shell := ["powershell.exe", "-c"]

export DATABASE_URL := "sqlite:schema.db"

run-back:
	cargo run --bin nndb-backend

serve-front:
	trunk serve -d target/dist frontend/index.html

hx:
	hx