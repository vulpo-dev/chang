/*
 * $1 string - queue
 * $2 u16 - limit
*/
with available_tasks as (
 	select id, state
 	  from chang.tasks all_tasks
 	 where all_tasks.queue = $1
 	   and all_tasks.scheduled_at <= now()
 	   and (all_tasks.state = 'available' or all_tasks.state = 'retryable')
 	   and case
 	         when depends_on is null then true
 	         else not exists (
 	         	select *
 	         	  from chang.tasks
 	         	 where dependend_id = all_tasks.depends_on
 	         	   and ( 
 	         	   	     state = 'running'
 	         	      or state = 'scheduled'
 	         	      or state = 'available'
 	         	      or state = 'retryable'
 	         	   )
 	         )
 	       end
 	 order by priority desc
 	        , scheduled_at asc
            , id asc
 	 limit $2
 	 for update skip locked
), insert_history as (
	insert into chang.task_history(task_id, from_state, to_state)
	select id as task_id
	     , available_tasks.state as from_state 
	     , 'running' as to_state
	  from available_tasks
	returning task_id as id
)
update chang.tasks
   set state = 'running'
     , attempt = attempt + 1
 where chang.tasks.id in (select * from insert_history)
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