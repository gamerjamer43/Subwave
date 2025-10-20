SELECT s.id, s.name, a.artist, a.name AS album, a.cover, s.duration, s.filename
FROM songs s

JOIN albums a ON s.album_id = a.id
WHERE a.id = $1
ORDER BY s.track_number ASC