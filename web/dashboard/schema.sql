-- schema.sql
DROP TABLE IF EXISTS sensor_readings;

CREATE TABLE sensor_readings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device TEXT NOT NULL,
    topic TEXT NOT NULL,
    temperature REAL NOT NULL,
    humidity REAL NOT NULL,
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

