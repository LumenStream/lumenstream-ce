CREATE TABLE IF NOT EXISTS account_permission_groups (
    id UUID PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS account_permission_group_playback_domains (
    group_id UUID NOT NULL REFERENCES account_permission_groups(id) ON DELETE CASCADE,
    domain_id UUID NOT NULL REFERENCES playback_domains(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (group_id, domain_id)
);

CREATE INDEX IF NOT EXISTS idx_account_permission_group_playback_domains_domain_id
    ON account_permission_group_playback_domains(domain_id);

ALTER TABLE billing_plans
    ADD COLUMN IF NOT EXISTS permission_group_id UUID;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'fk_billing_plans_permission_group_id'
    ) THEN
        ALTER TABLE billing_plans
            ADD CONSTRAINT fk_billing_plans_permission_group_id
            FOREIGN KEY (permission_group_id)
            REFERENCES account_permission_groups(id)
            ON DELETE SET NULL;
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_billing_plans_permission_group_id
    ON billing_plans(permission_group_id);
