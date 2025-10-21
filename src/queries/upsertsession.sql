INSERT INTO sessions (username, token, issued) VALUES ($1, $2, $3)
ON CONFLICT (username) DO UPDATE SET token = EXCLUDED.token, issued = EXCLUDED.issued