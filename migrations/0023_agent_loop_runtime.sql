ALTER TABLE agent_requests
ADD COLUMN IF NOT EXISTS public_state JSONB NOT NULL DEFAULT '{}'::jsonb;

ALTER TABLE agent_requests
ADD COLUMN IF NOT EXISTS runtime_state JSONB NOT NULL DEFAULT '{}'::jsonb;

ALTER TABLE agent_requests
ADD COLUMN IF NOT EXISTS current_round INTEGER NOT NULL DEFAULT 0;

ALTER TABLE agent_requests
ADD COLUMN IF NOT EXISTS max_rounds INTEGER NOT NULL DEFAULT 10;

ALTER TABLE agent_requests
ADD COLUMN IF NOT EXISTS public_phase TEXT NOT NULL DEFAULT 'queued';

ALTER TABLE agent_requests
ADD COLUMN IF NOT EXISTS waiting_for_user BOOLEAN NOT NULL DEFAULT false;

ALTER TABLE agent_requests
ADD COLUMN IF NOT EXISTS pending_question JSONB NULL;

ALTER TABLE agent_requests
ADD COLUMN IF NOT EXISTS question_deadline TIMESTAMPTZ NULL;

ALTER TABLE agent_request_events
ADD COLUMN IF NOT EXISTS visibility TEXT NOT NULL DEFAULT 'public';

ALTER TABLE agent_request_events
ADD COLUMN IF NOT EXISTS channel TEXT NOT NULL DEFAULT 'timeline';

CREATE INDEX IF NOT EXISTS idx_agent_requests_waiting_for_user
    ON agent_requests(waiting_for_user, question_deadline DESC);
