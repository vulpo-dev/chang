
with insert_history as (
	insert into chang.task_history(task_id, from_state, to_state)
	select $1 as task_id
	     , chang.tasks.state as from_state 
	     , 'running' as to_state
      from chang.tasks
     where id = $1 
       and state = 'scheduled' 
	returning task_id as id
)
update chang.tasks
   set state = 'running'
     , attempted_at = now()
     , attempt = attempt + 1
 where id in (select * from insert_history)
returning id
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
