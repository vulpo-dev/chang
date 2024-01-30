/*
 * $1 json - Array of logs 
 */
 
insert into chang.tasks(max_attempts, scheduled_at, priority, args, attempted_by, kind, queue, tags, depends_on, dependend_id)
select *
  from jsonb_to_recordset($1) as tasks
          ( max_attempts smallint 
          , scheduled_at timestamptz
          , priority smallint
          , args jsonb
          , attempted_by text[]
          , kind text
          , queue text
          , tags varchar(255)[]
          , depends_on uuid
          , dependend_id uuid
          )
returning id
