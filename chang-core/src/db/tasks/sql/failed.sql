
with insert_error as (
	insert into chang.task_error(task_id, error)
	select $1 as task_id
	     , $2 as error
	returning task_id
), insert_history as (
	insert into chang.task_history(task_id, from_state, to_state)
	select id as task_id
	     , 'running'::chang.tasks_state as from_state 
	     , case
	          when attempt < max_attempts
	          then 'retryable'::chang.tasks_state
	          else 'discarded'::chang.tasks_state
	       end as to_state
	  from chang.tasks
	 where id in (select * from insert_error)
	returning task_id, to_state
)
update chang.tasks
   set state = insert_history.to_state
  from insert_history
 where id = insert_history.task_id
