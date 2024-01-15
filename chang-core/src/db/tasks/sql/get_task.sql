
select id
     , state as "state: TaskState"
     , attempt
     , scheduled_at
     , max_attempts
     , attempted_by
     , tags
     , kind
     , args
     , priority
     , queue
     , depends_on
     , dependend_id
  from chang.tasks
 where id = $1
