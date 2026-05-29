-- rollup's INSERT (last day) and DELETE (older than 30 days) filter on time alone;
-- without this they full-scan server_polls every run. range queries can now seek.
CREATE INDEX idx_server_polls_time ON server_polls(time);
