INSERT INTO projects (name, repo)
VALUES ('decomp.me', 'https://decomp.me');

INSERT INTO versions (name, platform, project_id)
VALUES ('n64', 0, (SELECT id FROM projects WHERE name = 'decomp.me'));

INSERT INTO versions (name, platform, project_id)
VALUES ('psx', 1, (SELECT id FROM projects WHERE name = 'decomp.me'));

INSERT INTO versions (name, platform, project_id)
VALUES ('ps2', 2, (SELECT id FROM projects WHERE name = 'decomp.me'));

INSERT INTO versions (name, platform, project_id)
VALUES ('gc_wii', 3, (SELECT id FROM projects WHERE name = 'decomp.me'));

INSERT INTO versions (name, platform, project_id)
VALUES ('psp', 4, (SELECT id FROM projects WHERE name = 'decomp.me'));

INSERT INTO versions (name, platform, project_id)
VALUES ('gba', 5, (SELECT id FROM projects WHERE name = 'decomp.me'));

INSERT INTO versions (name, platform, project_id)
VALUES ('nds', 6, (SELECT id FROM projects WHERE name = 'decomp.me'));

INSERT INTO versions (name, platform, project_id)
VALUES ('n3ds', 7, (SELECT id FROM projects WHERE name = 'decomp.me'));

INSERT INTO versions (name, platform, project_id)
VALUES ('irix', 8, (SELECT id FROM projects WHERE name = 'decomp.me'));