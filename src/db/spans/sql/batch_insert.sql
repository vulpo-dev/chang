/*
 * $1 json - Array of logs 
 */
insert into chang.spans
select *
  from jsonb_to_recordset($1) as span_records
       ( id uuid 
       , trace_id text
       , span_id text
       , parent_span_id text
       , name text
       , kind chang.span_kind
       , start_time timestamptz
       , end_time timestamptz
       , attributes jsonb
       , resource jsonb
       , scope jsonb
       , events jsonb
       , links jsonb
       , status jsonb
       , dropped_attributes_count bigint
       , dropped_events_count bigint
       , dropped_links_count bigint
       , trace_state text
       )
on conflict (id) do update set id = uuid_generate_v4()