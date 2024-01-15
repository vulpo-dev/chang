
with insert_history as (
	insert into chang.task_history(task_id, from_state, to_state)
	select chang.tasks.id as task_id
	     , chang.tasks.state as from_state 
	     , $2 as to_state
	  from chang.tasks
	 where id = $1
	returning task_id as id
)
update chang.tasks
   set state = $2
 where id in (select * from insert_history)
