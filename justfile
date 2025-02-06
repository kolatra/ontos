set dotenv-load

alias r  := run
alias t  := test-all
alias b  := build
alias tp := test-all-print
alias ti := test-image
alias gp := ghcr-push
alias pr := push-release

push-release: 
	just gp europa && \
	just gp voyager && \
	just b ganymede

build NAME: (test NAME)
	docker compose build base && \
	docker image rm -f ghcr.io/kolatra/{{NAME}} && \
	docker compose build {{NAME}}

ghcr-push NAME: (build NAME)
	docker push ghcr.io/kolatra/{{NAME}}:latest

build-all: (test-all)
	docker compose build

test NAME:
	cargo clippy -p {{NAME}} -- -D warnings && \
	cargo test -p {{NAME}}

test-all:
	cargo clippy -- -D warnings && \
	cargo test 

test-all-print:
	cargo test -- --nocapture

test-image NAME: (build NAME)
	docker run -it --rm --env-file .env \
	--network="container:postgres-ontos" \
	--name {{NAME}}-test \
	-e RUST_LOG={{NAME}},common \
	ghcr.io/kolatra/{{NAME}}:latest

run TARGET:
	RUST_LOG={{TARGET}},common cargo run --bin {{TARGET}}

export-db:
	docker exec -t postgres-ontos pg_dumpall -c -U ontos > dump_`date +%d-%m-%Y"_"%H_%M_%S`.sql

sorm-gen:
	sea-orm-cli generate entity -u $DATABASE_URL -o crates/common/src/db/entities

postgres:
	docker compose -f docker-compose.db.yml up -d

migrate:
	sea-orm-cli migrate up

mc-s-flat port='25565':
	docker run -d -it --rm --name cli-server -p {{port}}:25565 \
	-e EULA=TRUE -e ALLOW_FLIGHT=false -e ONLINE_MODE=false \
	-e LEVEL_TYPE=minecraft:flat \
	itzg/minecraft-server

mc-s port='25565':
	docker run -d -it --rm --name cli-server -p {{port}}:25565 \
	-e EULA=TRUE -e ALLOW_FLIGHT=false -e ONLINE_MODE=true \
	itzg/minecraft-server

stop-s:
	docker stop cli-server

stats:
	curl -H "auth: ${API_KEY}" -l "127.0.0.1:$WEBSERVER_PORT/stats"

up:
	curl -H "auth: ${API_KEY}" -l "127.0.0.1:$WEBSERVER_PORT/query?column=protocol&value=761" > output.json

rs:
	curl -H "auth: ${ADMIN_KEY}" -l "127.0.0.1:$WEBSERVER_PORT/rs"

dive NAME='europa':
	docker run --rm -it \
	-v /var/run/docker.sock:/var/run/docker.sock \
	-e DOCKER_API_VERSION=1.37 \
	wagoodman/dive:latest ghcr.io/kolatra/{{NAME}}
