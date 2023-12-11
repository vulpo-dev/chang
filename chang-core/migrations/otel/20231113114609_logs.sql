-- Add migration script here

create table if not exists chang.logs
	( id uuid primary key default uuid_generate_v4()
	, severity_number bigint not null
	, dropped_attributes_count bigint not null default 0
	, observed_time timestamptz not null default now()
	, resource jsonb not null default '{}'::jsonb
	, scope jsonb not null default '{}'::jsonb
	, attributes jsonb not null default '{}'::jsonb
	, flags smallint
	, time timestamptz
	, severity_text text
	, body text
	, span_id text
	, trace_id text
	);

create index chang_logs_span_id_idx on chang.logs using btree(span_id);
create index chang_logs_trace_id_idx on chang.logs using btree(trace_id);
create index chang_logs_severity_number_idx on chang.logs using btree(severity_number);
create index chang_logs_scope_idx on chang.logs using gin(scope);
create index chang_logs_resource_idx on chang.logs using gin(resource);
create index chang_logs_attributes_idx on chang.logs using gin(attributes);
