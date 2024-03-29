
with insert_history as (
	insert into chang.task_history(task_id, from_state, to_state)
	select $1 as task_id
	     , 'running' as from_state 
	     , 'completed' as to_state
	returning task_id as id
)
update chang.tasks
   set state = 'completed'
 where id in (select * from insert_history)
