
insert into chang.tasks(max_attempts, scheduled_at, priority, args, attempted_by, kind, queue, tags, depends_on, dependend_id)
values (
	$1 -- max_attempts
  , coalesce($2, now()) -- scheduled_at
  , $3 -- priority
  , $4 -- args
  , $5 -- attempted_by
  , $6 -- kind
  , coalesce($7, 'default') -- queue
  , $8 -- tags
  , $9 -- depends_on
  , $10 -- dependend_id
  )
returning id