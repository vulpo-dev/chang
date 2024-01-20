create type chang.tasks_state as enum(
  'available',
  'cancelled',
  'completed',
  'discarded',
  'retryable',
  'running',
  'scheduled'
);

create table if not exists chang.tasks
	( id uuid primary key default uuid_generate_v4()
	, state chang.tasks_state not null default 'available'::chang.tasks_state
	, attempt smallint not null default 0
	, max_attempts smallint not null default 3
	, attempted_at timestamptz
	, created_at timestamptz not null default now()
	, scheduled_at timestamptz not null default now()
	, priority smallint not null default 1
	, args jsonb
	, attempted_by text[] not null default '{}'
	, kind text not null
	, metadata jsonb not null default '{}'::jsonb
	, queue text not null default 'default'::text
	, tags varchar(255)[]

	, depends_on uuid
	, dependend_id uuid

	, constraint max_attempts_is_positive check (max_attempts > 0)
	);

create index chang_task_kind on chang.tasks using btree(kind);
create index chang_task_depends_on on chang.tasks using btree(depends_on);
create index chang_task_dependend_id_state on chang.tasks using btree(dependend_id, state);
create index chang_task_args_index on chang.tasks using gin(args);
create index chang_task_metadata_index on chang.tasks using gin(metadata);
create index chang_task_get_tasks on chang.tasks using btree(queue, scheduled_at, state, id);
create index chang_task_priority on chang.tasks using btree(priority);
create index chang_task_scheduled_at on chang.tasks using btree(scheduled_at);
	
create table if not exists chang.task_history
	( id uuid primary key default uuid_generate_v4()
	, task_id uuid not null references chang.tasks(id) on delete cascade
	, from_state chang.tasks_state not null
	, to_state chang.tasks_state not null
	, comment text not null default ''
	, created_at timestamptz not null default now()
	);

create table if not exists chang.task_error
		( id uuid primary key default uuid_generate_v4()
		, task_id uuid not null references chang.tasks(id) on delete cascade
		, error text
		, created_at timestamptz not null default now()
		);


create table if not exists chang.periodic_tasks
	( id uuid primary key default uuid_generate_v4()
	, queue text not null default 'default'::text
	, kind text not null
	, schedule text not null
	, unique(queue, kind)
	);

create index chang_periodic_tasks_queue_kind_idx on chang.tasks using btree(queue, kind);
