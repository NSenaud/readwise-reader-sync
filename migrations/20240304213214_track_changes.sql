-- Inspired by https://github.com/iloveitaly/audit-trigger/blob/master/sql

-- This audit table is not secured as it can be altered by a user editing the
-- source table. The goal is only to keep a change history.
CREATE TABLE history (
    event_id BIGSERIAL PRIMARY KEY,
    table_name TEXT NOT NULL,
    relid OID NOT NULL,
    session_user_name TEXT,
    action_tstamp_tx TIMESTAMP WITH TIME ZONE NOT NULL,
    action_tstamp_stm TIMESTAMP WITH TIME ZONE NOT NULL,
    action_tstamp_clk TIMESTAMP WITH TIME ZONE NOT NULL,
    transaction_id BIGINT,
    client_addr INET,
    client_port INTEGER,
    client_query TEXT,
    action TEXT NOT NULL CHECK (action IN ('I', 'D', 'U', 'T')),
    row_data JSONB,
    changed_fields JSONB,
    statement_only BOOLEAN NOT NULL,
    row_id TEXT
);

COMMENT ON TABLE history IS 'History of auditable actions on audited tables, from if_modified_func()';
COMMENT ON COLUMN history.event_id IS 'Unique identifier for each auditable event';
COMMENT ON COLUMN history.table_name IS 'Non-schema-qualified table name of table event occurred in';
COMMENT ON COLUMN history.relid IS 'Table OID. Changes with drop/create. Get with ''tablename''::regclass';
COMMENT ON COLUMN history.session_user_name IS 'Login / session user whose statement caused the audited event';
COMMENT ON COLUMN history.action_tstamp_tx IS 'Transaction start timestamp for tx in which audited event occurred';
COMMENT ON COLUMN history.action_tstamp_stm IS 'Statement start timestamp for tx in which audited event occurred';
COMMENT ON COLUMN history.action_tstamp_clk IS 'Wall clock time at which audited event''s trigger call occurred';
COMMENT ON COLUMN history.transaction_id IS 'Identifier of transaction that made the change. May wrap, but unique paired with action_tstamp_tx.';
COMMENT ON COLUMN history.client_addr IS 'IP address of client that issued query. Null for unix domain socket.';
COMMENT ON COLUMN history.client_port IS 'Remote peer IP port address of client that issued query. Undefined for unix socket.';
COMMENT ON COLUMN history.client_query IS 'Top-level query that caused this auditable event. May be more than one statement.';
COMMENT ON COLUMN history.action IS 'Action type; I = insert, D = delete, U = update, T = truncate';
COMMENT ON COLUMN history.row_data IS 'Record value. Null for statement-level trigger. For INSERT this is the new tuple. For DELETE and UPDATE it is the old tuple.';
COMMENT ON COLUMN history.changed_fields IS 'New values of fields changed by UPDATE. Null except for row-level UPDATE events.';
COMMENT ON COLUMN history.statement_only IS '''t'' if audit event is from an FOR EACH STATEMENT trigger, ''f'' for FOR EACH ROW';

CREATE INDEX logged_actions_relid_idx ON history(relid);
CREATE INDEX logged_actions_action_tstamp_tx_stm_idx ON history(action_tstamp_stm);
CREATE INDEX logged_actions_action_idx ON history(action);

CREATE OR REPLACE FUNCTION if_modified_func() RETURNS TRIGGER AS $body$
DECLARE
    audit_row history;
    include_values boolean;
    log_diffs boolean;
    h_old jsonb;
    h_new jsonb;
    excluded_cols text [] = ARRAY []::text [];
    BEGIN IF TG_WHEN <> 'AFTER' THEN RAISE EXCEPTION 'if_modified_func() may only run as an AFTER trigger';
END IF;
audit_row = ROW(
    nextval('history_event_id_seq'), -- event_id
    TG_TABLE_NAME::text,             -- table_name
    TG_RELID,                        -- relation OID for much quicker searches
    session_user::text,              -- session_user_name
    current_timestamp,               -- action_tstamp_tx
    statement_timestamp(),           -- action_tstamp_stm
    clock_timestamp(),               -- action_tstamp_clk
    txid_current(),                  -- transaction ID
    inet_client_addr(),              -- client_addr
    inet_client_port(),              -- client_port
    current_query(),                 -- top-level query or queries (if multistatement) from client
    substring(TG_OP, 1, 1),          -- action
    NULL,
    NULL,                            -- row_data, changed_fields
    'f',                             -- statement_only,
    COALESCE(OLD.id, NULL)           -- pk ID of the row
);
IF NOT TG_ARGV [0]::boolean IS DISTINCT
FROM 'f'::boolean THEN audit_row.client_query = NULL;
END IF;
IF TG_ARGV [1] IS NOT NULL THEN excluded_cols = TG_ARGV [1]::text [];
END IF;
IF (
    TG_OP = 'UPDATE'
    AND TG_LEVEL = 'ROW'
) THEN audit_row.row_data = row_to_json(OLD)::JSONB - excluded_cols;
--Computing differences
SELECT jsonb_object_agg(tmp_new_row.key, tmp_new_row.value) AS new_data INTO audit_row.changed_fields
FROM jsonb_each_text(row_to_json(NEW)::JSONB) AS tmp_new_row
    JOIN jsonb_each_text(audit_row.row_data) AS tmp_old_row ON (
        tmp_new_row.key = tmp_old_row.key
        AND tmp_new_row.value IS DISTINCT
        FROM tmp_old_row.value
    );
IF audit_row.changed_fields = '{}'::JSONB THEN -- All changed fields are ignored. Skip this update.
RETURN NULL;
END IF;
ELSIF (
    TG_OP = 'DELETE'
    AND TG_LEVEL = 'ROW'
) THEN audit_row.row_data = row_to_json(OLD)::JSONB - excluded_cols;
ELSIF (
    TG_OP = 'INSERT'
    AND TG_LEVEL = 'ROW'
) THEN audit_row.row_data = row_to_json(NEW)::JSONB - excluded_cols;
ELSIF (
    TG_LEVEL = 'STATEMENT'
    AND TG_OP IN ('INSERT', 'UPDATE', 'DELETE', 'TRUNCATE')
) THEN audit_row.statement_only = 't';
ELSE RAISE EXCEPTION '[if_modified_func] - Trigger func added as trigger for unhandled case: %, %',
TG_OP,
TG_LEVEL;
RETURN NULL;
END IF;
INSERT INTO history
VALUES (audit_row.*);
RETURN NULL;
END;
$body$ LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog,
    public;
COMMENT ON FUNCTION if_modified_func() IS $body$ Track changes to a table at the statement
and /
or row level.Optional parameters to trigger in CREATE TRIGGER call: param 0: boolean,
whether to log the query text.Default 't'.param 1: text [],
columns to ignore in updates.Default [].Updates to ignored cols are omitted
from changed_fields.Updates with only ignored cols changed are not inserted into the audit log.Almost all the processing work is still done for updates that ignored.If you need to save the load,
    you need to use
    WHEN clause on the trigger instead.No warning
    or error is issued if ignored_cols contains columns that do not exist in the target table.This lets you specify a standard
set of ignored columns.There is no parameter to disable logging of
values.
Add this trigger as a 'FOR EACH STATEMENT' rather than 'FOR EACH ROW' trigger if you do not want to log row
values.Note that the user name logged is the login role for the session.The audit trigger cannot obtain the active role because it is reset by the SECURITY DEFINER invocation of the audit trigger its self.$body$;
CREATE OR REPLACE FUNCTION audit_table(
        target_table regclass,
        audit_rows boolean,
        audit_query_text boolean,
        audit_inserts boolean,
        ignored_cols text []
    ) RETURNS void AS $body$
DECLARE stm_targets text = 'INSERT OR UPDATE OR DELETE OR TRUNCATE';
_q_txt text;
_ignored_cols_snip text = '';
BEGIN PERFORM deaudit_table(target_table);
IF audit_rows THEN IF array_length(ignored_cols, 1) > 0 THEN _ignored_cols_snip = ', ' || quote_literal(ignored_cols);
END IF;
_q_txt = 'CREATE TRIGGER audit_trigger_row AFTER ' || CASE
    WHEN audit_inserts THEN 'INSERT OR '
    ELSE ''
END || 'UPDATE OR DELETE ON ' || target_table || ' FOR EACH ROW EXECUTE PROCEDURE if_modified_func(' || quote_literal(audit_query_text) || _ignored_cols_snip || ');';
RAISE NOTICE '%',
_q_txt;
EXECUTE _q_txt;
stm_targets = 'TRUNCATE';
ELSE
END IF;
_q_txt = 'CREATE TRIGGER audit_trigger_stm AFTER ' || stm_targets || ' ON ' || target_table || ' FOR EACH STATEMENT EXECUTE PROCEDURE if_modified_func(' || quote_literal(audit_query_text) || ');';
RAISE NOTICE '%',
_q_txt;
EXECUTE _q_txt;
END;
$body$ language 'plpgsql';
COMMENT ON FUNCTION audit_table(regclass, boolean, boolean, boolean, text []) IS $body$
Add auditing support to a table.Arguments: target_table: Table name,
    schema qualified if not on search_path audit_rows: Record each row change,
    or only audit at a statement level audit_query_text: Record the text of the client query that triggered the audit event ? audit_inserts: Audit
insert statements
    or only updates / deletes / truncates ? ignored_cols: Columns to exclude
from
update diffs,
    ignore updates that change only ignored cols.$body$;
-- Adaptor to older variant without the audit_inserts parameter for backwards compatibility
CREATE OR REPLACE FUNCTION audit_table(
        target_table regclass,
        audit_rows boolean,
        audit_query_text boolean,
        ignored_cols text []
    ) RETURNS void AS $body$
SELECT audit_table($1, $2, $3, BOOLEAN 't', ignored_cols);
$body$ LANGUAGE SQL;
-- Pg doesn't allow variadic calls with 0 params, so provide a wrapper
CREATE OR REPLACE FUNCTION audit_table(
        target_table regclass,
        audit_rows boolean,
        audit_query_text boolean,
        audit_inserts boolean
    ) RETURNS void AS $body$
SELECT audit_table($1, $2, $3, $4, ARRAY []::text []);
$body$ LANGUAGE SQL;
-- Older wrapper for backwards compatibility
CREATE OR REPLACE FUNCTION audit_table(
        target_table regclass,
        audit_rows boolean,
        audit_query_text boolean
    ) RETURNS void AS $body$
SELECT audit_table($1, $2, $3, BOOLEAN 't', ARRAY []::text []);
$body$ LANGUAGE SQL;
-- And provide a convenience call wrapper for the simplest case
-- of row-level logging with no excluded cols and query logging enabled.
--
CREATE OR REPLACE FUNCTION audit_table(target_table regclass) RETURNS void AS $body$
SELECT audit_table($1, BOOLEAN 't', BOOLEAN 't', BOOLEAN 't');
$body$ LANGUAGE 'sql';
COMMENT ON FUNCTION audit_table(regclass) IS $body$
Add auditing support to the given table.Row - level changes will be logged with full client query text.No cols are ignored.$body$;
CREATE OR REPLACE FUNCTION deaudit_table(target_table regclass) RETURNS void AS $body$ BEGIN EXECUTE 'DROP TRIGGER IF EXISTS audit_trigger_row ON ' || target_table;
EXECUTE 'DROP TRIGGER IF EXISTS audit_trigger_stm ON ' || target_table;
END;
$body$ language 'plpgsql';
COMMENT ON FUNCTION deaudit_table(regclass) IS $body$ Remove auditing support to the given table.$body$;
CREATE OR REPLACE VIEW tableslist AS
SELECT DISTINCT triggers.trigger_schema AS schema,
    triggers.event_object_table AS auditedtable
FROM information_schema.triggers
WHERE triggers.trigger_name::text IN (
        'audit_trigger_row'::text,
        'audit_trigger_stm'::text
    )
ORDER BY schema,
    auditedtable;
COMMENT ON VIEW tableslist IS $body$ View showing all tables with auditing
set up.Ordered by schema,
    then table.$body$;

-- Enable history on reading table
SELECT audit_table('reading');
