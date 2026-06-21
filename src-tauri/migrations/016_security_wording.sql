-- Remove the misleading implication that repository verification is read-only.
UPDATE tool
SET description = 'Run a repository-controlled verification command after explicit approval',
    risk_level = 'medium',
    requires_permission = 1
WHERE name = 'shell.verify';
