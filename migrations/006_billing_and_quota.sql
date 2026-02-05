-- Migration: Add billing system and user-level quota
-- Created: 2024

-- 1. Add quota-related fields to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS rate_limit_rpm INTEGER;
ALTER TABLE users ADD COLUMN IF NOT EXISTS monthly_quota DECIMAL(12, 6);
ALTER TABLE users ADD COLUMN IF NOT EXISTS current_month_usage DECIMAL(12, 6) DEFAULT 0;
ALTER TABLE users ADD COLUMN IF NOT EXISTS usage_reset_at TIMESTAMPTZ DEFAULT NOW();

-- 2. Create billings table for periodic billing records
CREATE TABLE IF NOT EXISTS billings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL,
    total_cost DECIMAL(12, 6) NOT NULL,
    total_tokens BIGINT NOT NULL DEFAULT 0,
    total_requests BIGINT NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    items JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    paid_at TIMESTAMPTZ,
    UNIQUE(user_id, period_start, period_end)
);

CREATE INDEX IF NOT EXISTS idx_billings_user_id ON billings(user_id);
CREATE INDEX IF NOT EXISTS idx_billings_status ON billings(status);
CREATE INDEX IF NOT EXISTS idx_billings_period ON billings(period_start, period_end);

-- 3. Create user_balances table for optional prepaid model
CREATE TABLE IF NOT EXISTS user_balances (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    balance DECIMAL(12, 6) NOT NULL DEFAULT 0,
    credit_limit DECIMAL(12, 6) NOT NULL DEFAULT 0,
    lifetime_usage DECIMAL(12, 6) NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 4. Add composite index for efficient usage queries by user and month
CREATE INDEX IF NOT EXISTS idx_usage_user_month ON usage(user_id, created_at DESC);
