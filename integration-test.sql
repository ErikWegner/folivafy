DROP DATABASE IF EXISTS inttest;
DROP ROLE IF EXISTS inttest_role;
CREATE ROLE inttest_role WITH LOGIN PASSWORD 'inttest_pwd';
CREATE DATABASE inttest WITH OWNER inttest_role;
GRANT ALL PRIVILEGES ON DATABASE inttest TO inttest_role;
