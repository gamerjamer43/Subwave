WITH album_upsert AS (
    INSERT INTO albums (name, artist, cover, runtime, songcount)
    VALUES ($1, $2, $3, 0, 0)
    ON CONFLICT (name, artist) DO UPDATE
    SET cover = EXCLUDED.cover
    RETURNING id
),

song_insert AS (
    INSERT INTO songs (name, album_id, track_number, duration, filename)
    VALUES (
        $4,
        (SELECT id FROM album_upsert),
        $5,
        $6,
        $7
    )

    ON CONFLICT (filename) DO UPDATE
    SET name = EXCLUDED.name,
        album_id = EXCLUDED.album_id,
        track_number = EXCLUDED.track_number,
        duration = EXCLUDED.duration
    RETURNING album_id
)

UPDATE albums
SET songcount = songcount + 1, runtime = runtime + $6
WHERE id = (SELECT album_id FROM song_insert);