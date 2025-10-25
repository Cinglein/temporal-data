CREATE OR REPLACE VIEW tx_density_hourly AS
SELECT
    date_trunc('hour', ts) AS hour_start,
    COUNT(*)               AS tx_count
FROM txs
GROUP BY 1
ORDER BY 1;

CREATE OR REPLACE VIEW tx_net_profit_hourly AS
SELECT
    date_trunc('hour', ts)                 AS hour_start,
    SUM(profit - fee)                      AS net_profit
FROM txs
GROUP BY 1
ORDER BY 1;
