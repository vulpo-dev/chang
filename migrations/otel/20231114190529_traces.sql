-- Add migration script here

create type chang.span_kind as enum('unspecified', 'internal', 'server', 'client', 'producer', 'consumer');

create table if not exists chang.spans
	( id uuid primary key default uuid_generate_v4()
	, trace_id text not null
	, span_id text not null
	, parent_span_id text not null
	, name text not null
	, kind chang.span_kind not null
	, start_time timestamptz not null
	, end_time timestamptz not null
	, attributes jsonb not null default '{}'::jsonb
	, resource jsonb not null default '{}'::jsonb
	, scope jsonb not null default '{}'::jsonb
	, events jsonb not null
	, links jsonb not null
	, status jsonb not null
	, dropped_attributes_count bigint not null default 0
	, dropped_events_count bigint not null default 0
	, dropped_links_count bigint not null default 0
	, trace_state text
	);

create index chang_spans_span_id_idx on chang.spans using btree(span_id);
create index chang_spans_trace_id_idx on chang.spans using btree(trace_id);
create index chang_spans_parent_span_id_idx on chang.spans using btree(parent_span_id);
create index chang_spans_name_idx on chang.spans using btree(name);
create index chang_spans_kind_idx on chang.spans using btree(kind);
create index chang_spans_resource_idx on chang.spans using gin(resource);
create index chang_spans_scope_idx on chang.spans using gin(scope);
create index chang_spans_attributes_idx on chang.spans using gin(attributes);
create index chang_spans_events_idx on chang.spans using gin(events);
