SELECT
    s.id AS id,
    s.name AS name,
    a.artist AS artist,
    a.name AS album,
    NULL::text AS "cover?",
    s.duration AS duration,
    s.filename AS filename
FROM songs s
JOIN albums a ON s.album_id = a.id
WHERE s.name ILIKE $1 OR a.artist ILIKE $1 OR a.name ILIKE $1
LIMIT 50;
