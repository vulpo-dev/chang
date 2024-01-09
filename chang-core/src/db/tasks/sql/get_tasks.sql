with available_tasks as (
 	select id
 	  from chang.tasks all_tasks
 	 where all_tasks.queue = 'default'
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
 	 order by priority asc
 	        , scheduled_at asc
 	 limit 10
 	 for update skip locked
), insert_history as (
	insert into chang.task_history(task_id, from_state, to_state)
	select id as task_id
	     , 'available'::chang.tasks_state as from_state 
	     , 'scheduled'::chang.tasks_state as to_state
	  from available_tasks
	returning task_id as id
)
update chang.tasks
   set state = 'scheduled'
 where chang.tasks.id in (select * from insert_history)
 returning id
