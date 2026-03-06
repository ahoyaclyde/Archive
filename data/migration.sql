-- Add Sessions Table to FLUG Evidence Database
-- Run this if the sessions table doesn't exist

-- Create sessions table
CREATE TABLE IF NOT EXISTS sessions (
    session_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    user_email TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME NOT NULL,
    last_activity DATETIME DEFAULT CURRENT_TIMESTAMP,
    ip_address TEXT,
    user_agent TEXT,
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires ON sessions(expires_at);
CREATE INDEX IF NOT EXISTS idx_sessions_email ON sessions(user_email);

-- Verify table was created
SELECT 
    name, 
    type,
    sql
FROM sqlite_master 
WHERE type='table' 
    AND name='sessions';

-- Show the indexes
SELECT 
    name,
    tbl_name,
    sql
FROM sqlite_master 
WHERE type='index' 
    AND tbl_name='sessions';

-- Display success message
SELECT 
    '✅ Sessions table created successfully!' as status,
    datetime('now') as created_at;