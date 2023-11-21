/*
 * $1 json - Array of logs 
 */
 
insert into chang.events
select *
  from jsonb_to_recordset($1) as event_records
       ( id uuid
       , kind text
       , body jsonb
       , created_at timestamptz
       )
on conflict (id) do update set id = uuid_generate_v4()
