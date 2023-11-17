/*
 * $1 json - Array of logs 
 */
insert into chang.metrics
select *
  from jsonb_to_recordset($1) as log_records
       ( id uuid 
       , name text
       , description text
       , data jsonb
       , resource jsonb
       , scope jsonb
       )
on conflict (id) do update set id = uuid_generate_v4()