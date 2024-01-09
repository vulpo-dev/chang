
with insert_history as (
	insert into chang.task_history(task_id, from_state, to_state)
	select $1 as task_id
	     , 'scheduled'::chang.tasks_state as from_state 
	     , 'running'::chang.tasks_state as to_state
	returning task_id as id
)
update chang.tasks
   set state = 'running'
     , attempted_at = now()
     , attempt = attempt + 1
 where id in (select * from insert_history)
returning id
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
