
create table if not exists chang.events
	( id uuid primary key default uuid_generate_v4()
	, kind text not null
	, body jsonb not null
	, created_at timestamptz not null default now()
	, inserted_at timestamptz not null default now()
	);

create index chang_events_body_idx on chang.events using gin(body);
create index chang_events_created_at_idx on chang.events using btree(created_at);
create index chang_events_inserted_at_idx on chang.events using btree(inserted_at);
