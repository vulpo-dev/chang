
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
 where kind = $1
   and queue = $2
 order by scheduled_at desc
 limit $3