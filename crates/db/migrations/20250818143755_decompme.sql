INSERT INTO projects (name, repo)
VALUES ('decomp.me', 'https://decomp.me');

INSERT INTO versions (name, platform, project_id)
VALUES ('n64', 0, (SELECT id FROM projects WHERE name = 'decomp.me'));

INSERT INTO versions (name, platform, project_id)
VALUES ('psx', 1, (SELECT id FROM projects WHERE name = 'decomp.me'));

INSERT INTO versions (name, platform, project_id)
VALUES ('ps2', 2, (SELECT id FROM projects WHERE name = 'decomp.me'));

INSERT INTO versions (name, platform, project_id)
VALUES ('gc', 3, (SELECT id FROM projects WHERE name = 'decomp.me'));

INSERT INTO versions (name, platform, project_id)
VALUES ('wii', 4, (SELECT id FROM projects WHERE name = 'decomp.me'));

INSERT INTO versions (name, platform, project_id)
VALUES ('psp', 5, (SELECT id FROM projects WHERE name = 'decomp.me'));