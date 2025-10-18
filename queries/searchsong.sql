SELECT s.id as id, s.name as name, a.artist as artist, a.name as album, s.duration as duration, s.filename as filename
FROM songs s

JOIN albums a ON s.album_id = a.id
WHERE s.name ILIKE $1 OR a.artist ILIKE $1 OR a.name ILIKE $1
LIMIT 50;