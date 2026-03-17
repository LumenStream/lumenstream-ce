ALTER TABLE agent_requests
RENAME COLUMN moviepilot_payload TO provider_payload;

ALTER TABLE agent_requests
RENAME COLUMN moviepilot_result TO provider_result;
