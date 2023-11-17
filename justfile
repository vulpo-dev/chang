set shell := ["pwsh", "-c"]
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

database-create:
	sqlx database create

database-setup: database-create && migrate

database-drop:
	sqlx database drop

database-reset: database-drop database-setup

sqlx-prepare:
	cargo sqlx prepare

migrate-otel:
	sqlx migrate run --source ./migrations/otel --ignore-missing

migrate-base: 
	sqlx migrate run --source ./migrations/base --ignore-missing

migrate: migrate-base migrate-otel

example-logs:
	cargo run --example logs

example-trace:
	cargo run --example trace

example-metrics:
	cargo run --example metrics

example-tracing-otel:
	cargo run --example tracing-otel

up:
	docker compose up -d

down:
	docker compose stop