SELECT
    s.id AS id,
    s.name AS name,
    a.artist AS artist,
    a.name AS album,
    a.cover AS "cover?",
    s.duration AS duration,
    s.filename AS filename
FROM songs s

JOIN albums a ON s.album_id = a.id
WHERE a.id = $1
ORDER BY s.track_number ASC
