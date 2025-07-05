CREATE OR REPLACE FUNCTION notify_event(channel_name text)
  RETURNS trigger AS $$
BEGIN
  PERFORM pg_notify(channel_name, '');
  RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER on_insert_notify_event_pbs_computations
AFTER INSERT ON pbs_computations
FOR EACH STATEMENT EXECUTE FUNCTION notify_event('event_pbs_computations');

CREATE TRIGGER on_insert_notify_event_allowed_handle
AFTER INSERT ON allowed_handles
FOR EACH STATEMENT EXECUTE FUNCTION notify_event('event_allowed_handle');
