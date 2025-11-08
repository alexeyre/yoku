-- This file should undo anything in `up.sql`

-- Delete test exercises
DELETE FROM Exercises
WHERE name IN (
    'Push-up',
    'Pull-up',
    'Squat',
    'Deadlift',
    'Bench Press',
    'Overhead Press',
    'Bicep Curl',
    'Tricep Extension',
    'Plank',
    'Lunge'
);

-- Delete test tags
DELETE FROM Tags
WHERE name IN (
    'left-side',
    'right-side',
    'both-sides',
    'slow and controlled',
    'to failure',
    'sick',
    'warm-up',
    'cool-down',
    'explosive',
    'assistance'
);
