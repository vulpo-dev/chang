
create table if not exists chang.metrics
	( id uuid primary key default uuid_generate_v4()
	, name text not null
	, description text not null
	, data jsonb
	, resource jsonb not null default '{}'::jsonb
	, scope jsonb not null default '{}'::jsonb
	);

create index chang_metrics_name_idx on chang.metrics using btree(name);
create index chang_metrics_data_idx on chang.metrics using gin(data);
create index chang_metrics_scope_idx on chang.metrics using gin(scope);
create index chang_metrics_resource_idx on chang.metrics using gin(resource);
