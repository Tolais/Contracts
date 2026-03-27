-- Database schema for analytics system

-- Creator analytics data (aggregated daily)
CREATE TABLE IF NOT EXISTS creator_analytics (
    id SERIAL PRIMARY KEY,
    creator_id VARCHAR(255) NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    revenue DECIMAL(20, 8) NOT NULL DEFAULT 0,
    active_streams INTEGER NOT NULL DEFAULT 0,
    cancellations INTEGER NOT NULL DEFAULT 0,
    new_subscribers INTEGER NOT NULL DEFAULT 0,
    total_subscribers INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Index for efficient queries
CREATE INDEX idx_creator_analytics_creator_timestamp 
ON creator_analytics(creator_id, timestamp DESC);

-- Revenue streams table
CREATE TABLE IF NOT EXISTS revenue_streams (
    id SERIAL PRIMARY KEY,
    creator_id VARCHAR(255) NOT NULL,
    stream_name VARCHAR(255) NOT NULL,
    monthly_value DECIMAL(20, 8) NOT NULL DEFAULT 0,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    started_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMP WITH TIME ZONE,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes for revenue streams
CREATE INDEX idx_revenue_streams_creator ON revenue_streams(creator_id);
CREATE INDEX idx_revenue_streams_status ON revenue_streams(status);

-- Function to update the updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Trigger to auto-update updated_at
CREATE TRIGGER update_revenue_streams_updated_at
    BEFORE UPDATE ON revenue_streams
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- View for creator statistics
CREATE OR REPLACE VIEW creator_stats AS
SELECT 
    creator_id,
    COUNT(DISTINCT CASE WHEN status = 'active' THEN id END) as active_stream_count,
    SUM(CASE WHEN status = 'active' THEN monthly_value ELSE 0 END) as total_monthly_recurring_revenue,
    AVG(CASE WHEN status = 'active' THEN monthly_value END) as avg_stream_value,
    COUNT(DISTINCT CASE WHEN status = 'cancelled' THEN id END) as churned_streams,
    COALESCE(
        COUNT(DISTINCT CASE WHEN status = 'cancelled' THEN id END)::FLOAT / 
        NULLIF(COUNT(DISTINCT id), 0),
        0
    ) as churn_rate
FROM revenue_streams
GROUP BY creator_id;

-- Sample data insertion (for testing)
-- Uncomment for development/testing
/*
INSERT INTO creator_analytics (creator_id, timestamp, revenue, active_streams, cancellations)
SELECT 
    'creator_' || i,
    NOW() - (random() * INTERVAL '90 days'),
    (random() * 1000 + 500)::DECIMAL,
    (random() * 20 + 5)::INTEGER,
    (random() * 3)::INTEGER
FROM generate_series(1, 50) AS i;
*/
