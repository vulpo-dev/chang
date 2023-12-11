/*
 * $1 json - Array of logs 
 */
insert into chang.logs
select *
  from jsonb_to_recordset($1) as log_records
       ( id uuid 
       , severity_number bigint
       , dropped_attributes_count bigint
       , observed_time timestamptz
       , resource jsonb
       , scope jsonb
       , attributes jsonb
       , flags smallint
       , time timestamptz
       , severity_text text
       , body text
       , span_id text
       , trace_id text
       )
on conflict (id) do update set id = uuid_generate_v4()