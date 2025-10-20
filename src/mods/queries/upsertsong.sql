INSERT INTO songs (name, album_id, track_number, duration, filename)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT (filename) DO UPDATE
SET name = EXCLUDED.name,
    album_id = EXCLUDED.album_id,
    track_number = EXCLUDED.track_number,
    duration = EXCLUDED.duration;