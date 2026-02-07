-- Backfill user_balances for existing users who don't have a record
-- This ensures all users have a balance record for prepaid model enforcement

INSERT INTO user_balances (user_id)
SELECT id FROM users
WHERE id NOT IN (SELECT user_id FROM user_balances)
ON CONFLICT (user_id) DO NOTHING;
