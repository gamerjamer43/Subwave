SELECT s.id as id, s.name as name, a.artist as artist, a.name as album, s.duration as duration, s.filename as filename
FROM songs s

JOIN albums a ON s.album_id = a.id
WHERE s.name LIKE ? OR a.artist LIKE ? OR a.name LIKE ?
LIMIT 50;