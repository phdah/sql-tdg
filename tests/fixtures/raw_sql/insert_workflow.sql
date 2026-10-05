CREATE TABLE insert_sink AS
SELECT value
FROM raw_insert
WHERE 1 = 0;

INSERT INTO insert_sink
SELECT value
FROM raw_insert
WHERE value = 10;

CREATE VIEW insert_result AS
SELECT value
FROM insert_sink
WHERE value = 10
ORDER BY value;
