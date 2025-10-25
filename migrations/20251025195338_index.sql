CREATE INDEX idx_txs_ts ON txs(ts);
CREATE INDEX idx_txs_feepayer ON txs(feepayer);
CREATE UNIQUE INDEX idx_txs_signature ON txs(signature);
